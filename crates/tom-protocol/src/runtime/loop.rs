/// The protocol runtime event loop — thin orchestrator.
///
/// Owns RuntimeState + TomNode. Multiplexes over transport events,
/// application commands, and timers. Delegates all logic to RuntimeState,
/// executes effects via executor.
use tokio::sync::{broadcast, mpsc};
use ed25519_dalek::Signer;
use tom_transport::{BootstrapHint, TomNode};

use crate::discovery::DiscoverySource;
use crate::types::{now_ms, NodeId};

use super::bootstrap::{BootstrapPhase, BootstrapSource};
use super::effect::RuntimeEffect;
use super::executor::execute_effects;
use super::state::{GossipInput, RuntimeState};
use super::{DeliveredMessage, ProtocolEvent, RuntimeCommand};
use crate::tracker::StatusChange;

use tom_gossip::Gossip;
use tom_gossip::api::{Event as GossipEvent, GossipSender};
use n0_future::StreamExt;
use tom_connect::TransportAddr;
use tom_transport::PathEvent;

use super::metrics::ProtocolMetrics;

/// Fixed gossip topic for ToM peer discovery (all nodes share this).
const TOM_GOSSIP_TOPIC: [u8; 32] = *b"tom-protocol-gossip-discovery-v1";

/// Silence window after which "connected" peers are treated as zombies.
/// Healthy peers emit gossip announces (~10s) + heartbeats, so 45s of total
/// inbound silence means the links are dead even if QUIC still reports them.
const LIVENESS_STALE_MS: u64 = 45_000;

/// Whether inbound traffic has gone stale (zombie-connection detector).
/// Saturating so a `last_inbound` slightly in the future never underflows to stale.
fn liveness_is_stale(last_inbound: u64, now: u64, threshold_ms: u64) -> bool {
    now.saturating_sub(last_inbound) > threshold_ms
}

/// Interval for maintenance timers — after a stall (CPU contention, iOS resume,
/// or weak device strain), we want ONE tick of rattrapage, not a burst of all
/// missed ticks. Skip realigns the schedule without replaying the backlog.
/// This prevents orage republish/rejoin storms during recovery.
fn maintenance_interval(period: std::time::Duration) -> tokio::time::Interval {
    let mut i = tokio::time::interval(period);
    i.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    i
}

/// Main event loop — thin orchestrator.
///
/// All protocol logic lives in `RuntimeState`. This function only:
/// 1. Multiplexes I/O events via `tokio::select!`
/// 2. Calls the appropriate `RuntimeState` method
/// 3. Feeds resulting effects to the executor
#[allow(clippy::too_many_arguments)]
pub(super) async fn runtime_loop(
    mut node: TomNode,
    mut state: RuntimeState,
    gossip_bootstrap_peers: Vec<NodeId>,
    cmd_tx: mpsc::Sender<RuntimeCommand>,
    mut cmd_rx: mpsc::Receiver<RuntimeCommand>,
    msg_tx: mpsc::Sender<DeliveredMessage>,
    status_tx: mpsc::Sender<StatusChange>,
    event_tx: mpsc::Sender<ProtocolEvent>,
    mut path_rx: broadcast::Receiver<PathEvent>,
    gossip: Gossip,
    metrics: ProtocolMetrics,
) {
    // ── Timers (read intervals from state.config) ───────────────────
    let mut cache_cleanup = maintenance_interval(state.config.cache_cleanup_interval);
    let mut tracker_cleanup = maintenance_interval(state.config.tracker_cleanup_interval);
    let mut heartbeat_check = maintenance_interval(state.config.heartbeat_interval);
    let mut group_hub_heartbeat = maintenance_interval(state.config.group_hub_heartbeat_interval);
    let mut backup_tick = maintenance_interval(state.config.backup_tick_interval);
    let mut gossip_announce = maintenance_interval(state.config.gossip_announce_interval);
    let mut shadow_ping = maintenance_interval(state.config.shadow_ping_interval);
    let mut subnet_eval = maintenance_interval(std::time::Duration::from_secs(30));
    let mut role_eval = maintenance_interval(std::time::Duration::from_secs(300));
    let mut state_save = maintenance_interval(std::time::Duration::from_secs(30));
    let mut dht_republish = maintenance_interval(std::time::Duration::from_secs(30 * 60));
    let mut delivery_deadline = maintenance_interval(std::time::Duration::from_secs(5));
    let mut hub_cleanup = maintenance_interval(std::time::Duration::from_secs(60));
    let mut reconnect_check = maintenance_interval(std::time::Duration::from_secs(15));
    // Shared DHT rendezvous: periodic re-announce + zero-config peer discovery.
    let mut rendezvous_tick = maintenance_interval(std::time::Duration::from_secs(60));
    // L1-001 presence: purge 30s-TTL artifacts every 5s (ephemeral, LOCKED #2).
    let mut presence_purge = maintenance_interval(std::time::Duration::from_secs(5));
    // L1-001 presence auto-probe (fleet observability). When disabled the
    // timer still exists but its tick handler is a no-op (config-checked).
    let mut presence_probe = maintenance_interval(
        state
            .config
            .presence_probe_interval
            .unwrap_or(std::time::Duration::from_secs(3600)),
    );
    // L1-003: relay-side presence view publication (30s push, D2).
    let mut presence_publish = maintenance_interval(std::time::Duration::from_secs(30));
    // Phase R18.1: Backup redelivery queue drain (1s interval for throughput)
    let mut backup_drain_tick = maintenance_interval(std::time::Duration::from_secs(1));

    // ── Containment des rafales de join (#34/#35) ─────────────────────
    // TOUTES les sources de découverte (mDNS, DHT/rendezvous, PeerPresent)
    // faisaient rejoindre un pair sans borne. Deux dégâts observés terrain :
    // (1) un pair connecté re-joint toutes les ~5 s (ré-annonce mDNS) → rafale ;
    // (2) le rendezvous DHT accumule des node_ids MORTS (taille_reseau ~350
    // pour 4 pairs réels) et l'iPhone DIAL en boucle vers ces fantômes →
    // CPU 100 % + chauffe. Parade : un cooldown de join PAR PAIR, partagé
    // entre toutes les sources, conditionné à l'état de connexion —
    //   • pair connecté      → 60 s (anti-rafale en régime établi) ;
    //   • pair NON connecté  → 20 s (retry borné : converge vite pour un pair
    //     vivant, mais ne martèle jamais un fantôme mort).
    // `join_cooldown_ok` met à jour la carte quand il autorise. Purgé au
    // PeerOffline (comme peer_paths).
    let mut join_cooldown: std::collections::HashMap<NodeId, std::time::Instant> =
        std::collections::HashMap::new();

    // Skip the immediate first tick
    cache_cleanup.tick().await;
    tracker_cleanup.tick().await;
    heartbeat_check.tick().await;
    group_hub_heartbeat.tick().await;
    backup_tick.tick().await;
    gossip_announce.tick().await;
    shadow_ping.tick().await;
    subnet_eval.tick().await;
    role_eval.tick().await;
    state_save.tick().await;
    dht_republish.tick().await;
    delivery_deadline.tick().await;
    hub_cleanup.tick().await;
    reconnect_check.tick().await;
    rendezvous_tick.tick().await;
    presence_purge.tick().await;
    presence_probe.tick().await;
    backup_drain_tick.tick().await;
    presence_publish.tick().await;

    // ── Gossip subscription ──────────────────────────────────────────
    let topic_id = tom_gossip::TopicId::from_bytes(TOM_GOSSIP_TOPIC);
    let bootstrap: Vec<tom_connect::EndpointId> = gossip_bootstrap_peers
        .iter()
        .map(|n| *n.as_endpoint_id())
        .collect();

    let (gossip_sender, mut gossip_receiver) = match gossip.subscribe(topic_id, bootstrap).await {
        Ok(topic) => {
            let (s, r) = topic.split();
            tracing::info!("gossip: subscribed to discovery topic");
            (Some(s), Some(r))
        }
        Err(e) => {
            tracing::warn!("gossip: subscription failed: {e}");
            (None, None)
        }
    };

    // ── Discovery timing instrumentation (Phase R12.1) ──────────────────
    // Capture the exact moment the runtime loop starts to track cold-start delay.
    let discovery_start = std::time::Instant::now();
    tracing::info!("⏱️ DISCO t+0 : runtime_loop started");
    let _ = event_tx
        .send(ProtocolEvent::DiscoveryTiming {
            elapsed_ms: 0,
            detail: "🚀 runtime_loop started".to_string(),
        })
        .await;

    // ── DHT setup ──────────────────────────────────────────────────
    let secret_seed = node.secret_key_seed();
    // Clone the async DHT handle for spawned lookup tasks (cheap Arc clone)
    let dht_handle: Option<tom_dht::AsyncDht> =
        state.dht().map(|d| d.async_dht());

    // Publish to DHT at startup (BEP-0044) — EN TÂCHE DÉTACHÉE.
    // Le put_mutable DHT prend des dizaines de secondes ; le faire en bloquant
    // ici retardait d'autant l'entrée dans la boucle select! → la découverte
    // LAN mDNS (quasi instantanée) était prise en otage, convergence à ~28 s.
    // Détaché : on entre dans la boucle immédiatement, le mDNS trouve les pairs
    // du LAN en 1-2 s, et le publish DHT se termine en fond.
    {
        let (relay_urls, direct_addrs) = extract_node_addrs(&node);
        state.publish_to_dht_detached(&secret_seed, relay_urls, direct_addrs);
        tracing::info!("⏱️ DISCO t+0 : DHT startup publish lancé (détaché)");
        let _ = event_tx
            .send(ProtocolEvent::DiscoveryTiming {
                elapsed_ms: 0,
                detail: "DHT startup publish lancé (détaché)".to_string(),
            })
            .await;
    }

    // Monotonic floor for rendezvous publication timestamps (see build_self_dht_addr).
    let mut rendezvous_ts_floor: u64 = 0;

    // Liveness: timestamp of the last inbound traffic (any message or gossip
    // event). Used to detect ZOMBIE connections — peers that QUIC still reports
    // as "connected" but that have gone silent. connected_peers() alone can't
    // see this, so a stale inbound clock means we are effectively isolated.
    let mut last_inbound_at = now_ms();


    // Throttle the mass-rejoin triggered on gossip NeighborDown. Without it, a
    // flapping neighbor (or several at once) fires join_peers(ALL known peers)
    // dozens of times per second — a rejoin storm that burns CPU on weak
    // devices (Apple TV) and floods the activity log with PeerStale churn.
    let mut last_gossip_rejoin_at: u64 = 0;
    const GOSSIP_REJOIN_THROTTLE_MS: u64 = 3_000;
    // Rejoin only peers seen within this window (exclude stale DHT-rendezvous
    // noise), capped as a hard backstop against fan-out on weak devices.
    const REJOIN_RECENT_WINDOW_MS: u64 = 5 * 60 * 1_000;
    const REJOIN_MAX_PEERS: usize = 16;

    // Announce into the shared rendezvous + pull live peers immediately, so a
    // node with no prior knowledge starts finding the network from the first tick.
    spawn_rendezvous_round(
        &node,
        &state.local_id,
        dht_handle.as_ref(),
        &cmd_tx,
        &mut rendezvous_ts_floor,
        &secret_seed,
        &state.config.username,
        state.config.app_build,
        discovery_start,
        event_tx.clone(),
        true, // startup round
    );

    // ── PeerPresent receiver from relay ────────────────────────────────
    let mut peer_present_rx = node.take_peer_present_rx();
    let mut bootstrap_hint_rx = node.take_bootstrap_hint_rx();
    let mut bootstrap_phase = BootstrapPhase::LanProbe;

    // Cheap cloneable send handle sharing `node`'s connection pool — network
    // sends are spawned off this instead of the executor borrowing `node`
    // directly, so a slow/hanging send to an unreachable peer never blocks
    // this loop's own ticks (see execute_effects doc comment).
    let node_sender = node.sender();

    // ── Embedded relay service ─────────────────────────────────────────
    let mut embedded_relay = super::EmbeddedRelayService::new();
    // UPnP mapping watcher for the embedded relay (Phase R13). Held stable across
    // select! iterations — a `watch::Receiver` recreated every tick (via a fresh
    // `watch_mapped_address()` call inside the select branch) would silently miss
    // any mapping change that lands between two loop iterations, since a freshly
    // constructed receiver only observes *future* changes from its creation point.
    let mut embedded_relay_mapped_watcher: Option<
        tokio::sync::watch::Receiver<Option<std::net::SocketAddrV4>>,
    > = None;

    // Auto-start embedded relay if enabled in config
    if state.config.enable_embedded_relay {
        let relay_config = super::EmbeddedRelayConfig {
            bind_addr: state.config.embedded_relay_bind_addr,
            advertise_addr: state.config.embedded_relay_advertise_addr,
        };
        let relay_start = std::time::Instant::now();
        let startup_effects = match embedded_relay.start(relay_config).await {
            Ok(url) => {
                // Chronométré : ce start pré-boucle est le dernier await réseau
                // avant le select! — s'il dure, TOUTE la découverte est figée
                // (hints mDNS non traités). Mesuré pour trancher, pas supposer.
                tracing::info!(
                    "⏱️ DISCO t+{}ms : relai embarqué démarré (start en {}ms)",
                    discovery_start.elapsed().as_millis(),
                    relay_start.elapsed().as_millis()
                );
                // Grab the watcher BEFORE any other await point: a mapping that
                // resolves during reprobe_relays() must still be observed by this
                // receiver's initial seen_version, not silently skipped.
                embedded_relay_mapped_watcher = embedded_relay.watch_mapped_address();
                // Détaché : un netcheck complet peut durer >1 min quand des
                // relais/DNS sont injoignables — l'awaiter ici bloquait la
                // boucle ~115 s sur iOS (flotte figée au restart simultané).
                node.reprobe_relays_detached();
                state.handle_command(super::RuntimeCommand::EmbeddedRelayStarted { url })
            }
            Err(error) => state.handle_command(super::RuntimeCommand::EmbeddedRelayFailed { error }),
        };
        execute_effects(startup_effects, &node_sender, &msg_tx, &status_tx, &event_tx, &metrics);
    }

    // ── Rejoin groups after restart (deferred until connectivity) ─────
    // The Join + SyncRequest must go out only once we actually have a live path
    // to the hub. Firing here — pre-loop, before any connection exists — would
    // emit them into the void and the R13 offline gap-fill would silently no-op
    // on a cold restart. Instead we arm a flag and (re)try in reconnect_check.
    let mut rejoin_pending = state.group_manager.group_count() > 0;

    // ── Transport relay discovery state (NOT in RuntimeState — pure transport concern)
    let mut discovered_transport_relays: std::collections::HashSet<tom_connect::RelayUrl> =
        std::collections::HashSet::new();

    // ── Main loop ────────────────────────────────────────────────────
    loop {
        let effects = tokio::select! {
            // ── 1. Incoming data from transport ─────────────────
            result = node.recv_raw() => {
                match result {
                    Ok((_from, data)) => {
                        metrics.inc_messages_received();
                        last_inbound_at = now_ms();
                        state.handle_incoming(&data)
                    }
                    Err(e) => vec![RuntimeEffect::Emit(ProtocolEvent::Error {
                        description: format!("recv error: {e}"),
                    })],
                }
            }

            // ── 2. Commands from application ────────────────────
            Some(cmd) = cmd_rx.recv() => {
                match cmd {
                    RuntimeCommand::GetConnectedPeers { reply } => {
                        let peers = node.connected_peers().await;
                        let _ = reply.send(peers);
                        Vec::new()
                    }
                    RuntimeCommand::AddPeerAddr { addr } => {
                        let node_id = NodeId::from_endpoint_id(addr.id);
                        bootstrap_join_peer(
                            &node,
                            gossip_sender.as_ref(),
                            addr,
                            BootstrapSource::Manual,
                            &mut bootstrap_phase,
                            discovery_start,
                            event_tx.clone(),
                        ).await;
                        state.handle_command(RuntimeCommand::AddPeer { node_id, source: DiscoverySource::Direct })
                    }
                    RuntimeCommand::AddPeer { node_id, source } => {
                        // Spawn a DHT lookup for unknown peers (non-blocking)
                        if let Some(dht_client) = dht_handle.as_ref() {
                            if state.topology.get(&node_id).is_none() {
                                let dht_clone = dht_client.clone();
                                let pk = node_id.as_bytes();
                                let tx = cmd_tx.clone();
                                tokio::spawn(async move {
                                    match tom_dht::dht_lookup(&dht_clone, &pk).await {
                                        Ok(Some(addr)) => {
                                            let _ = tx.send(
                                                RuntimeCommand::DhtLookupResult { addr }
                                            ).await;
                                        }
                                        Ok(None) => {}
                                        Err(e) => {
                                            tracing::debug!("DHT lookup failed: {e}");
                                        }
                                    }
                                });
                            }
                        }
                        state.handle_command(RuntimeCommand::AddPeer { node_id, source })
                    }
                    RuntimeCommand::DhtLookupResult { ref addr } => {
                        // Build EndpointAddr from DHT record and inject into transport.
                        // Cooldown de join (#35) : le rendezvous DHT ramène des
                        // node_ids MORTS (vieilles sessions) ; sans borne, on dial en
                        // boucle vers eux → CPU 100 % + chauffe (observé iPhone). Le
                        // cooldown non-connecté (20 s) borne ces tentatives.
                        if let Some(endpoint_addr) = dht_addr_to_endpoint_addr(addr) {
                            let node_id = NodeId::from_endpoint_id(endpoint_addr.id);
                            if join_cooldown_ok(&node_id, &mut join_cooldown, &state.peer_paths, std::time::Instant::now()) {
                                bootstrap_join_peer(
                                    &node,
                                    gossip_sender.as_ref(),
                                    endpoint_addr,
                                    BootstrapSource::Dht,
                                    &mut bootstrap_phase,
                                    discovery_start,
                                    event_tx.clone(),
                                ).await;
                            } else {
                                tracing::trace!(peer = %node_id, "DHT join ignoré (cooldown join)");
                            }
                        }
                        state.handle_command(cmd)
                    }
                    RuntimeCommand::StartEmbeddedRelay { config } => {
                        match embedded_relay.start(config).await {
                            Ok(url) => {
                                embedded_relay_mapped_watcher = embedded_relay.watch_mapped_address();
                                node.reprobe_relays().await;
                                state.handle_command(RuntimeCommand::EmbeddedRelayStarted { url })
                            }
                            Err(error) => {
                                state.handle_command(RuntimeCommand::EmbeddedRelayFailed { error })
                            }
                        }
                    }
                    RuntimeCommand::StopEmbeddedRelay => {
                        embedded_relay.stop().await;
                        embedded_relay_mapped_watcher = None;
                        state.handle_command(RuntimeCommand::EmbeddedRelayStopped)
                    }
                    RuntimeCommand::SaveState { reply } => {
                        state.save_state();
                        let _ = reply.send(());
                        Vec::new()
                    }
                    RuntimeCommand::Shutdown => break,
                    other => state.handle_command(other),
                }
            }

            // ── 3. Path events from transport ───────────────────
            Ok(event) = path_rx.recv() => {
                tracing::info!(
                    peer = %event.remote,
                    kind = %event.kind,
                    rtt_ms = event.rtt.as_millis(),
                    "path changed"
                );
                // Store path kind for backup redelivery gating (Phase R18.1)
                state.peer_paths.insert(event.remote, event.kind);
                vec![RuntimeEffect::Emit(ProtocolEvent::PathChanged { event })]
            }

            // ── 3a. LAN bootstrap hints (mDNS) ───────────────
            event = async {
                match bootstrap_hint_rx.as_mut() {
                    Some(rx) => rx.recv().await,
                    None => std::future::pending().await,
                }
            } => {
                if event.is_none() {
                    // Canal fermé (sender lâché) : cesser de le sélectionner. Sans ça,
                    // recv() renvoie None IMMÉDIATEMENT en boucle → cette branche fire
                    // en continu → spin 100% CPU (device brûlant + kill iOS pour conso
                    // injustifiée). Le `None => pending()` ci-dessus ne couvre que le
                    // cas "pas de canal", pas le cas "canal fermé".
                    tracing::debug!("bootstrap_hint fermé — arrêt de la sélection (anti-spin)");
                    bootstrap_hint_rx = None;
                    Vec::new()
                } else if let Some(BootstrapHint::MdnsDiscovered { endpoint_addr }) = event {
                    let node_id = NodeId::from_endpoint_id(endpoint_addr.id);
                    // Cooldown de join unifié (#34/#35) : borne le rejoin par pair
                    // selon l'état de connexion (voir join_cooldown_ok).
                    if !join_cooldown_ok(&node_id, &mut join_cooldown, &state.peer_paths, std::time::Instant::now()) {
                        tracing::trace!(peer = %node_id, "mDNS hint ignoré (cooldown join)");
                        Vec::new()
                    } else {
                        let elapsed = discovery_start.elapsed().as_millis() as u64;
                        // Trace dev uniquement — l'événement Live Log est émis par
                        // bootstrap_join_peer juste après (une ligne par join, pas deux).
                        tracing::info!("⏱️ DISCO t+{elapsed}ms : mDNS hint for peer {}", node_id);
                        bootstrap_join_peer(
                            &node,
                            gossip_sender.as_ref(),
                            endpoint_addr,
                            BootstrapSource::Mdns,
                            &mut bootstrap_phase,
                            discovery_start,
                            event_tx.clone(),
                        ).await;
                        state.handle_command(RuntimeCommand::AddPeer { node_id, source: DiscoverySource::Mdns })
                    }
                } else {
                    Vec::new()
                }
            }

            // ── 3b. Relay PeerPresent: auto-discovery ──────────
            event = async {
                match peer_present_rx.as_mut() {
                    Some(rx) => rx.recv().await,
                    None => std::future::pending().await,
                }
            } => {
                if event.is_none() {
                    // Canal fermé (ex. connexion relais tombée en cellulaire) : cesser
                    // de le sélectionner, sinon recv() renvoie None en boucle → spin
                    // 100% CPU. C'est le cas le plus probable de la chauffe observée
                    // sur un device isolé/en réseau instable.
                    tracing::debug!("peer_present fermé — arrêt de la sélection (anti-spin)");
                    peer_present_rx = None;
                    Vec::new()
                } else if let Some((endpoint_id, relay_url)) = event {
                    let node_id = NodeId::from_endpoint_id(endpoint_id);
                    // Même cooldown de join que mDNS/DHT (#34/#35) : cette source
                    // n'était PAS bornée — le relais ré-annonce les mêmes pairs et
                    // chaque hint relançait un join même pour un pair déjà connecté.
                    if !join_cooldown_ok(&node_id, &mut join_cooldown, &state.peer_paths, std::time::Instant::now()) {
                        tracing::trace!(peer = %node_id, "PeerPresent hint ignoré (cooldown join)");
                        Vec::new()
                    } else {
                        let elapsed = discovery_start.elapsed().as_millis() as u64;
                        // Trace dev uniquement — l'événement Live Log est émis par
                        // bootstrap_join_peer juste après (une ligne par join, pas deux).
                        tracing::info!("⏱️ DISCO t+{elapsed}ms : PeerPresent hint from relay for peer {}", node_id);
                        let addr = tom_connect::EndpointAddr::new(endpoint_id).with_relay_url(relay_url);
                        bootstrap_join_peer(
                            &node,
                            gossip_sender.as_ref(),
                            addr,
                            BootstrapSource::PeerPresent,
                            &mut bootstrap_phase,
                            discovery_start,
                            event_tx.clone(),
                        ).await;
                        state.handle_command(RuntimeCommand::AddPeer { node_id, source: DiscoverySource::PeerPresent })
                    }
                } else {
                    Vec::new()
                }
            }

            // ── 4. Timer: cache cleanup ─────────────────────────
            _ = cache_cleanup.tick() => state.tick_cache_cleanup(),

            // ── 5. Timer: tracker eviction ──────────────────────
            _ = tracker_cleanup.tick() => state.tick_tracker_cleanup(),

            // ── 6. Timer: heartbeat liveness check ──────────────
            _ = heartbeat_check.tick() => state.tick_heartbeat(),

            // ── 7. Timer: group hub heartbeat ───────────────────
            _ = group_hub_heartbeat.tick() => state.tick_group_hub_heartbeat(),

            // ── 7b. Timer: shadow ping watchdog ──────────────────
            _ = shadow_ping.tick() => state.tick_shadow_ping(),

            // ── 7c. Timer: hub message cleanup (24h TTL) ─────────
            _ = hub_cleanup.tick() => state.tick_hub_cleanup(),

            // ── 8. Timer: backup maintenance ────────────────────
            _ = backup_tick.tick() => state.tick_backup(),

            // ── 8a. Timer: backup redelivery queue drain (1s) ──────
            _ = backup_drain_tick.tick() => state.drain_backup_queue(),

            // ── 9. Gossip events ────────────────────────────────
            event = async {
                match gossip_receiver.as_mut() {
                    Some(rx) => rx.next().await,
                    None => std::future::pending::<Option<_>>().await,
                }
            } => {
                if event.is_none() {
                    // Flux gossip terminé (sender lâché) : cesser de le sélectionner,
                    // sinon next() renvoie None en boucle → spin 100% CPU.
                    tracing::debug!("flux gossip terminé — arrêt de la sélection (anti-spin)");
                    gossip_receiver = None;
                    Vec::new()
                } else if let Some(Ok(event)) = event {
                    // Any gossip event is proof of a live mesh — refresh liveness.
                    last_inbound_at = now_ms();
                    match event {
                        GossipEvent::Received(msg) => {
                            state.handle_gossip_event(
                                GossipInput::PeerAnnounce(msg.content.to_vec())
                            )
                        }
                        GossipEvent::NeighborUp(endpoint_id) => {
                            let node_id = NodeId::from_endpoint_id(endpoint_id);
                            let effects = state.handle_gossip_event(
                                GossipInput::NeighborUp(node_id)
                            );
                            // A gossip neighbor = bootstrap complete, regardless of hint source
                            if bootstrap_phase != BootstrapPhase::Converged {
                                bootstrap_phase.on_hint_accepted();
                                let elapsed = discovery_start.elapsed().as_millis() as u64;
                                tracing::info!(
                                    peer = %node_id,
                                    "⏱️ DISCO t+{elapsed}ms : bootstrap CONVERGED via GossipNeighborUp"
                                );
                                let _ = event_tx
                                    .send(ProtocolEvent::DiscoveryTiming {
                                        elapsed_ms: elapsed,
                                        detail: format!("Voisin gossip {} → réseau convergé", node_id),
                                    })
                                    .await;
                            }
                            // Re-broadcast announce on NeighborUp
                            // (key learning from PoC-3: initial broadcast has no neighbors)
                            if let Some(ref sender) = gossip_sender {
                                if let Some(bytes) = state.build_gossip_announce() {
                                    let _ = sender.broadcast(bytes::Bytes::from(bytes)).await;
                                }
                            }
                            effects
                        }
                        GossipEvent::NeighborDown(endpoint_id) => {
                            let node_id = NodeId::from_endpoint_id(endpoint_id);
                            let effects = state.handle_gossip_event(GossipInput::NeighborDown(node_id));

                            // Re-bootstrap on neighbor loss: rejoin known peers so
                            // we reconnect instead of staying isolated — but THROTTLED.
                            // A flapping neighbor otherwise fires this dozens of times
                            // per second (rejoin storm). At most once per throttle
                            // window; flaps within the window are coalesced.
                            let now = now_ms();
                            if now.saturating_sub(last_gossip_rejoin_at)
                                > GOSSIP_REJOIN_THROTTLE_MS
                            {
                                if let Some(ref sender) = gossip_sender {
                                    // Only rejoin peers seen RECENTLY: the topology
                                    // holds hundreds of DHT-rendezvous entries that
                                    // are mostly noise (never actually dialable), so
                                    // rejoining all of them wastes dials on weak
                                    // devices. Cap as a hard backstop.
                                    let mut recent: Vec<_> = state.topology.peers()
                                        .filter(|p| now.saturating_sub(p.last_seen)
                                            < REJOIN_RECENT_WINDOW_MS)
                                        .map(|p| (p.last_seen, *p.node_id.as_endpoint_id()))
                                        .collect();
                                    // Most-recently-seen first, then cap.
                                    recent.sort_by_key(|(seen, _)| std::cmp::Reverse(*seen));
                                    let known_peers: Vec<_> = recent.into_iter()
                                        .take(REJOIN_MAX_PEERS)
                                        .map(|(_, id)| id)
                                        .collect();
                                    if !known_peers.is_empty() {
                                        last_gossip_rejoin_at = now;
                                        tracing::info!(
                                            peers = known_peers.len(),
                                            "neighbor down — rejoining {} recent peers (throttled)",
                                            known_peers.len()
                                        );
                                        let _ = sender.join_peers(known_peers).await;
                                    }
                                }
                            }

                            effects
                        }
                        GossipEvent::Lagged => {
                            tracing::warn!("gossip: receiver lagged, missed events");
                            Vec::new()
                        }
                    }
                } else {
                    Vec::new()
                }
            }

            // ── 10. Timer: subnet evaluation ────────────────────
            _ = subnet_eval.tick() => state.tick_subnets(),

            // ── 11. Timer: gossip announce ──────────────────────
            _ = gossip_announce.tick() => {
                if let Some(ref sender) = gossip_sender {
                    if let Some(bytes) = state.build_gossip_announce() {
                        if let Err(e) = sender.broadcast(bytes::Bytes::from(bytes)).await {
                            tracing::debug!("gossip: announce broadcast failed: {e}");
                        }
                    }
                }
                Vec::new()
            }

            // ── 12. Timer: role evaluation ──────────────────────
            _ = role_eval.tick() => state.tick_roles(),

            // ── 13. Timer: state persistence + metrics update ──
            _ = state_save.tick() => {
                state.save_state();
                metrics.set_groups_count(state.group_manager.group_count() as u64);
                metrics.set_peers_known(state.topology.len() as u64);
                // Fallback phase advance: relay-only nodes may never receive a mDNS/PeerPresent
                // hint, but they still discover peers via received messages. Converge only on
                // a LIVE (online) peer — a topology full of stale/offline entries means we are
                // isolated, not converged, and must stay in amorçage (see reconnect_check).
                if bootstrap_phase != BootstrapPhase::Converged && state.topology.online_count() > 0 {
                    bootstrap_phase.on_hint_accepted();
                    tracing::info!(
                        online = state.topology.online_count(),
                        "bootstrap: converged via live topology peer"
                    );
                }
                metrics.set_phase(bootstrap_phase);
                metrics.set_taille_reseau(state.topology.online_count() as u64);
                metrics.set_relayeurs_connus(state.topology.relay_count() as u64);
                metrics.set_role_local(state.topology.local_role(&state.local_id));
                Vec::new()
            }

            // ── 14. Timer: DHT re-publish (30 min) ───────────
            _ = dht_republish.tick() => {
                let (relay_urls, direct_addrs) = extract_node_addrs(&node);
                state.publish_to_dht(&secret_seed, relay_urls, direct_addrs).await;
                Vec::new()
            }

            // ── 15. Timer: delivery deadline check (5s) ────
            _ = delivery_deadline.tick() => state.tick_delivery_deadlines(),

            // ── 15a. Timer: presence purge (5s) — 30s TTL, never persisted ──
            _ = presence_purge.tick() => state.tick_presence_cleanup(),

            // ── 15b. Timer: presence auto-probe (config-gated, off by default) ──
            _ = presence_probe.tick() => state.tick_presence_probe(),

            // ── 15c. Timer: L1-003 presence view publication (30s push) ──
            _ = presence_publish.tick() => state.tick_publish_presence_views(),

            // ── 15b. Timer: DHT rendezvous (60s) — zero-config discovery ──
            _ = rendezvous_tick.tick() => {
                let elapsed = discovery_start.elapsed().as_millis() as u64;
                tracing::info!("⏱️ DISCO t+{elapsed}ms : 60s rendezvous tick");
                let _ = event_tx
                    .send(ProtocolEvent::DiscoveryTiming {
                        elapsed_ms: elapsed,
                        detail: "60s rendezvous tick (periodic)".to_string(),
                    })
                    .await;
                spawn_rendezvous_round(
                    &node,
                    &state.local_id,
                    dht_handle.as_ref(),
                    &cmd_tx,
                    &mut rendezvous_ts_floor,
                    &secret_seed,
                    &state.config.username,
                    state.config.app_build,
                    discovery_start,
                    event_tx.clone(),
                    false, // not startup
                );
                Vec::new()
            }

            // ── 16. Timer: reconnect known peers + isolation recovery (15s) ──
            // Periodically rejoin all known peers so that discovered-but-unconnected
            // peers get a fresh connection attempt. Cheap if already connected.
            // If we have ZERO live connections, treat it as isolation: re-enter the
            // bootstrap phase and actively re-run discovery instead of freezing in a
            // stale Converged state because a relay/peer disappeared.
            _ = reconnect_check.tick() => {
                // R13 cold-restart gap-fill: once connectivity is established
                // (topology shows online peers — connected_peers() is empty under
                // relay-only meshing, so it can't be the signal), (re)send Join +
                // SyncRequest so the hub streams the group messages missed while
                // offline. One-shot: cleared after the first connected attempt.
                if rejoin_pending && state.topology.online_count() > 0 {
                    let effects = state.build_rejoin_effects();
                    if !effects.is_empty() {
                        tracing::info!("R13: deferred group rejoin after connectivity");
                        execute_effects(
                            effects, &node_sender, &msg_tx, &status_tx, &event_tx, &metrics,
                        );
                    }
                    rejoin_pending = false;
                }
                if let Some(ref sender) = gossip_sender {
                    let known_node_ids: Vec<_> =
                        state.topology.peers().map(|p| p.node_id).collect();
                    let known_eids: Vec<_> =
                        known_node_ids.iter().map(|n| *n.as_endpoint_id()).collect::<Vec<_>>();

                    // FINDING #1 fix (red team) — every awaited network op in this
                    // recovery branch is BOUNDED. Previously they were awaited raw
                    // inside the `select!`, so a single slow/hung op (DHT publish,
                    // gossip join to dead peers, relay reprobe) froze the ENTIRE
                    // event loop: the node stopped draining commands and stopped
                    // replying to queries — a live-but-unresponsive "black hole"
                    // reproduced by chaos.monkey seed 7. These are all best-effort
                    // (results discarded, retried every 15s), so a timeout that lets
                    // the loop resume and drain its backlog is strictly better than
                    // an unbounded await. See docs/redteam/JOURNAL.md FINDING #1.
                    const RECOVERY_OP_TIMEOUT: std::time::Duration =
                        std::time::Duration::from_millis(1500);

                    // Isolation = no live QUIC connection OR connections that have
                    // gone silent (zombies): connected_peers() can't see a dead-but-
                    // open link, so a stale inbound clock is treated as isolation too.
                    // On timeout, treat as isolated (empty) — the safe default that
                    // triggers recovery rather than wedging the loop.
                    let connected =
                        tokio::time::timeout(RECOVERY_OP_TIMEOUT, node.connected_peers())
                            .await
                            .unwrap_or_default();
                    let zombie = !connected.is_empty()
                        && liveness_is_stale(last_inbound_at, now_ms(), LIVENESS_STALE_MS);
                    if connected.is_empty() || zombie {
                        if bootstrap_phase.on_isolated() {
                            tracing::info!(
                                connexions = connected.len(),
                                zombie,
                                "reconnect_check: isolé — retour en amorçage, redécouverte active"
                            );
                        }
                        // Reset the liveness clock so a zombie state doesn't re-fire
                        // every tick; give the fresh discovery a chance to reconnect.
                        last_inbound_at = now_ms();
                        // NB (#35) : on ne vide PLUS le cooldown de join ici. Le vider
                        // rejouait un rejoin de TOUS les pairs (fantômes compris) à
                        // chaque isolement → rafale/chauffe. Le cooldown non-connecté
                        // (20 s) redécouvre déjà assez vite sans marteler les morts.
                        // Re-announce ourselves and re-resolve known peers via DHT to
                        // pick up fresh addresses (old ones may be dead after a change).
                        let (relay_urls, direct_addrs) = extract_node_addrs(&node);
                        let _ = tokio::time::timeout(
                            RECOVERY_OP_TIMEOUT,
                            state.publish_to_dht(&secret_seed, relay_urls, direct_addrs),
                        )
                        .await;
                        // Zero-config recovery: hit the shared rendezvous NOW (don't wait
                        // for the 60s tick) to find live peers we never heard of.
                        let elapsed = discovery_start.elapsed().as_millis() as u64;
                        tracing::info!("⏱️ DISCO t+{elapsed}ms : isolation recovery — rendezvous NOW");
                        let _ = event_tx
                            .send(ProtocolEvent::DiscoveryTiming {
                                elapsed_ms: elapsed,
                                detail: "Isolation recovery: rendezvous NOW".to_string(),
                            })
                            .await;
                        spawn_rendezvous_round(
                            &node,
                            &state.local_id,
                            dht_handle.as_ref(),
                            &cmd_tx,
                            &mut rendezvous_ts_floor,
                            &secret_seed,
                            &state.config.username,
                            state.config.app_build,
                            discovery_start,
                            event_tx.clone(),
                            false, // not startup
                        );
                        if let Some(dht_client) = dht_handle.as_ref() {
                            for node_id in &known_node_ids {
                                let dht_clone = dht_client.clone();
                                let pk = node_id.as_bytes();
                                let tx = cmd_tx.clone();
                                tokio::spawn(async move {
                                    if let Ok(Some(addr)) = tom_dht::dht_lookup(&dht_clone, &pk).await {
                                        let _ = tx.send(RuntimeCommand::DhtLookupResult { addr }).await;
                                    }
                                });
                            }
                        }
                    }

                    if !known_eids.is_empty() {
                        let _ = tokio::time::timeout(
                            RECOVERY_OP_TIMEOUT,
                            sender.join_peers(known_eids),
                        )
                        .await;
                    }
                    // Always reprobe relays: triggers PeerPresent even when topology has
                    // known-but-offline peers (e.g. after relay cut or network change).
                    let _ = tokio::time::timeout(RECOVERY_OP_TIMEOUT, node.reprobe_relays()).await;
                }
                Vec::new()
            }

            // ── 17. UPnP port mapping change for embedded relay (Phase R13) ──
            // Observe the SAME stable watcher across iterations (see declaration
            // above) — recreating it here would silently drop any mapping change
            // that lands between two select! iterations (a fresh watch::Receiver
            // only sees changes from its own creation point onward).
            mapped_change = async {
                match embedded_relay_mapped_watcher.as_mut() {
                    Some(watcher) => watcher.changed().await,
                    None => std::future::pending().await,
                }
            }, if embedded_relay_mapped_watcher.is_some() => {
                if let Ok(()) = mapped_change {
                    if let Some(watcher) = embedded_relay_mapped_watcher.as_ref() {
                        if let Some(socket_addr_v4) = *watcher.borrow() {
                            // Convert SocketAddrV4 to SocketAddr (IPv4)
                            let external_addr = std::net::SocketAddr::V4(socket_addr_v4);
                            tracing::debug!("UPnP port mapping obtained: {external_addr}");
                            state.handle_command(RuntimeCommand::EmbeddedRelayPortMapped { external_addr })
                        } else {
                            Vec::new()
                        }
                    } else {
                        Vec::new()
                    }
                } else {
                    Vec::new()
                }
            }

            else => break,
        };

        // Intercept effects that need special handling (gossip, embedded relay)
        let mut regular_effects = Vec::with_capacity(effects.len());
        for effect in effects {
            match effect {
                RuntimeEffect::BroadcastRoleChange(ref announce) => {
                    if let Some(ref sender) = gossip_sender {
                        if let Ok(bytes) = rmp_serde::to_vec(announce) {
                            if let Err(e) = sender.broadcast(bytes::Bytes::from(bytes)).await {
                                tracing::debug!("gossip: role announce broadcast failed: {e}");
                            }
                        }
                    }
                }
                RuntimeEffect::StartEmbeddedRelay { config } => {
                    let feedback_effects = match embedded_relay.start(config).await {
                        Ok(url) => {
                            embedded_relay_mapped_watcher = embedded_relay.watch_mapped_address();
                            node.reprobe_relays().await;
                            state.handle_command(RuntimeCommand::EmbeddedRelayStarted { url })
                        }
                        Err(error) => state.handle_command(RuntimeCommand::EmbeddedRelayFailed { error }),
                    };
                    regular_effects.extend(feedback_effects);
                }
                RuntimeEffect::StopEmbeddedRelay => {
                    embedded_relay.stop().await;
                    embedded_relay_mapped_watcher = None;
                    let feedback_effects = state.handle_command(RuntimeCommand::EmbeddedRelayStopped);
                    regular_effects.extend(feedback_effects);
                }
                RuntimeEffect::BroadcastRelayReady(ref announce) => {
                    if let Some(ref sender) = gossip_sender {
                        if let Ok(bytes) = rmp_serde::to_vec(announce) {
                            if let Err(e) = sender.broadcast(bytes::Bytes::from(bytes)).await {
                                tracing::debug!("gossip: relay-ready broadcast failed: {e}");
                            }
                        }
                    }
                }
                RuntimeEffect::InsertTransportRelay { relay_url } => {
                    if !discovered_transport_relays.contains(&relay_url) {
                        // Conservative config: quic: None — discovery signal doesn't advertise QUIC
                        let config = std::sync::Arc::new(tom_connect::RelayConfig {
                            url: relay_url.clone(),
                            quic: None,
                        });
                        node.insert_relay(relay_url.clone(), config).await;
                        discovered_transport_relays.insert(relay_url.clone());
                        tracing::info!(%relay_url, "transport: inserted discovered relay");
                        let _ = event_tx.try_send(ProtocolEvent::TransportRelayInserted {
                            relay_url,
                        });
                    }
                }
                RuntimeEffect::RemoveTransportRelay { relay_url } => {
                    // Only remove if we inserted it via discovery (not static)
                    if discovered_transport_relays.remove(&relay_url) {
                        node.remove_relay(&relay_url).await;
                        tracing::info!(%relay_url, "transport: removed discovered relay");
                        let _ = event_tx.try_send(ProtocolEvent::TransportRelayRemoved {
                            relay_url,
                        });
                    }
                }
                other => {
                    regular_effects.push(other);
                }
            }
        }

        // Execute remaining effects
        execute_effects(regular_effects, &node_sender, &msg_tx, &status_tx, &event_tx, &metrics);

        // Sync topology metrics after every loop iteration
        metrics.set_taille_reseau(state.topology.online_count() as u64);
        metrics.set_peers_known(state.topology.len() as u64);
        metrics.set_relayeurs_connus(state.topology.relay_count() as u64);
        metrics.set_role_local(state.topology.local_role(&state.local_id));
        metrics.set_groups_count(state.group_manager.group_count() as u64);
    }

    // Save state before shutdown
    state.save_state();

    // Stop embedded relay if running
    embedded_relay.stop().await;

    // Graceful shutdown — bounded: a slow QUIC/relay close must never hang the
    // caller (the UI Stop blocks on the runtime teardown).
    match tokio::time::timeout(std::time::Duration::from_secs(2), node.shutdown()).await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!("runtime shutdown error: {e}"),
        Err(_) => tracing::warn!("runtime shutdown timed out after 2s — forcing"),
    }
}

/// Extract relay URLs and direct addresses from the TomNode for DHT publication.
fn extract_node_addrs(node: &TomNode) -> (Vec<String>, Vec<String>) {
    let addr = node.addr();
    let relay_urls: Vec<String> = addr
        .addrs
        .iter()
        .filter_map(|a| match a {
            TransportAddr::Relay(url) => Some(url.to_string()),
            _ => None,
        })
        .collect();
    let direct_addrs: Vec<String> = addr
        .addrs
        .iter()
        .filter_map(|a| match a {
            TransportAddr::Ip(sa) => Some(sa.to_string()),
            _ => None,
        })
        .collect();
    (relay_urls, direct_addrs)
}

/// Plafond du nombre d'adresses acceptées d'un enregistrement DHT (non fiable).
/// Un nœud légitime n'annonce qu'une poignée de relais/adresses ; un
/// enregistrement malveillant pourrait sinon injecter des milliers d'adresses
/// (anti-DoS : le parsing + l'insertion dans le BTreeSet restent bornés).
const MAX_DHT_ADDRS: usize = 32;

/// Convert a DHT node address to an EndpointAddr for transport injection.
fn dht_addr_to_endpoint_addr(addr: &tom_dht::DhtNodeAddr) -> Option<tom_connect::EndpointAddr> {
    let node_id: NodeId = addr.node_id.parse().ok()?;
    let mut addrs = std::collections::BTreeSet::new();

    for url_str in addr.relay_urls.iter().take(MAX_DHT_ADDRS) {
        if let Ok(url) = url_str.parse::<tom_connect::RelayUrl>() {
            addrs.insert(TransportAddr::Relay(url));
        }
    }
    for addr_str in addr.direct_addrs.iter().take(MAX_DHT_ADDRS) {
        if let Ok(sa) = addr_str.parse::<std::net::SocketAddr>() {
            // Drop never-dialable addresses injected via DHT (loopback/unspecified/
            // link-local): dialing them wastes attempts or hits local services.
            // KEEP private LAN ranges + public so same-LAN discovery via DHT works.
            if direct_addr_is_dialable(sa.ip()) {
                addrs.insert(TransportAddr::Ip(sa));
            }
        }
    }

    Some(tom_connect::EndpointAddr {
        id: *node_id.as_endpoint_id(),
        addrs,
    })
}

/// Whether a DHT-advertised direct IP is worth dialing. Rejects only the
/// never-routable classes (loopback, unspecified, link-local, broadcast,
/// documentation, multicast); private LAN ranges are kept (needed for same-LAN
/// connect). Multicast is rejected (red-team FINDING #13): a unicast dial target
/// is never multicast, so a poisoned rendezvous entry advertising one would only
/// waste connection slots / disturb multicast groups.
fn direct_addr_is_dialable(ip: std::net::IpAddr) -> bool {
    match ip {
        std::net::IpAddr::V4(v4) => {
            !(v4.is_loopback()
                || v4.is_unspecified()
                || v4.is_link_local()
                || v4.is_broadcast()
                || v4.is_multicast()
                || v4.is_documentation())
        }
        std::net::IpAddr::V6(v6) => {
            let seg = v6.segments();
            let link_local = (seg[0] & 0xffc0) == 0xfe80;
            !(v6.is_loopback() || v6.is_unspecified() || v6.is_multicast() || link_local)
        }
    }
}

/// Build this node's rendezvous record from its current transport addresses.
///
/// `ts_floor` makes the published timestamp monotonic per node: since the
/// rendezvous uses `seq = timestamp` (BEP-0044), a backward clock step (NTP,
/// VM resume) must NOT lower our seq — that would get our update silently
/// rejected and leave a stale address in our slot. We never publish a timestamp
/// below the last one we used. It stays wall-clock-aligned in steady state.
fn build_self_dht_addr(
    node: &TomNode,
    local_id: &NodeId,
    ts_floor: &mut u64,
    secret_seed: &[u8; 32],
    username: &str,
    app_build: u32,
) -> tom_dht::DhtNodeAddr {
    let (relay_urls, direct_addrs) = extract_node_addrs(node);
    let ts = now_ms().max(ts_floor.saturating_add(1));
    *ts_floor = ts;
    let mut addr = tom_dht::DhtNodeAddr {
        node_id: local_id.to_string(),
        relay_urls,
        direct_addrs,
        username: tom_dht::DhtNodeAddr::sanitize_username(username),
        app_build,
        timestamp: ts,
        sig: Vec::new(),
    };
    // Proof-of-possession: sign with our node key so readers can prove this
    // rendezvous entry really belongs to node_id (shared slot keys can't).
    let sig = ed25519_dalek::SigningKey::from_bytes(secret_seed).sign(&addr.signing_bytes());
    addr.sig = sig.to_bytes().to_vec();
    addr
}

/// Verify a rendezvous entry's proof-of-possession signature against its node_id.
/// Rejects unsigned, malformed, or forged entries (anti-squatting/poisoning).
/// Borne les tentatives de join par pair (#34/#35). Renvoie `true` (et note
/// l'instant) si un join est autorisé, `false` s'il est encore en cooldown.
/// Fenêtre : 60 s pour un pair connecté (anti-rafale), 20 s sinon (retry borné
/// — évite de marteler un node_id mort découvert via le rendezvous DHT).
fn join_cooldown_ok(
    node_id: &NodeId,
    cooldown: &mut std::collections::HashMap<NodeId, std::time::Instant>,
    peer_paths: &std::collections::HashMap<NodeId, tom_transport::PathKind>,
    now: std::time::Instant,
) -> bool {
    let connected = matches!(
        peer_paths.get(node_id),
        Some(tom_transport::PathKind::Direct) | Some(tom_transport::PathKind::Relay)
    );
    let window = if connected {
        std::time::Duration::from_secs(60)
    } else {
        // Pair NON connecté → retry rapide (5 s) pour une reconvergence <5 s
        // après un restart : si le 1er dial échoue (adresse/gossip pas prêts),
        // on re-tente au hint mDNS suivant (~5 s) au lieu d'attendre 20 s. Les
        // fantômes du rendezvous (vus toutes les 60 s) ne sont pas plus
        // sollicités — leur cadence est bornée par le tick rendezvous, pas ici.
        std::time::Duration::from_secs(5)
    };
    if cooldown
        .get(node_id)
        .is_some_and(|t| now.duration_since(*t) < window)
    {
        return false;
    }
    // Borne mémoire : sur une longue session le rendezvous DHT expose des
    // milliers de node_ids distincts. Quand la carte enfle, on évince les
    // entrées dont la fenêtre max (60 s) est écoulée — elles n'ont plus d'effet.
    if cooldown.len() > 4096 {
        cooldown.retain(|_, t| now.duration_since(*t) < std::time::Duration::from_secs(60));
    }
    cooldown.insert(*node_id, now);
    true
}

fn rendezvous_entry_authentic(addr: &tom_dht::DhtNodeAddr) -> bool {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    let Ok(node_id) = addr.node_id.parse::<NodeId>() else {
        return false;
    };
    let Ok(vk) = VerifyingKey::from_bytes(&node_id.as_bytes()) else {
        return false;
    };
    if addr.sig.len() != 64 {
        return false;
    }
    let mut sig_bytes = [0u8; 64];
    sig_bytes.copy_from_slice(&addr.sig);
    vk.verify(&addr.signing_bytes(), &Signature::from_bytes(&sig_bytes))
        .is_ok()
}

/// Âge max toléré d'une entrée du rendez-vous partagé. Un nœud VIVANT se
/// réannonce toutes les 60s (`rendezvous_tick`), donc son entrée a au plus
/// ~1 min. 10 min = 10× cette cadence : très généreux (absorbe ticks manqués,
/// réveils d'appareil, décalage d'horloge modéré) tout en bornant la durée de
/// vie d'un FANTÔME — un nœud mort (mon nœud de test, un appareil éteint) cesse
/// de se réannoncer, son entrée vieillit et n'est plus injectée passé ce délai.
const RENDEZVOUS_MAX_AGE_MS: u64 = 10 * 60 * 1000;

/// Une entrée rendez-vous est-elle FRAÎCHE (nœud probablement encore vivant) ?
/// Rejette les entrées clairement abandonnées (timestamp trop ancien). Un
/// timestamp dans le FUTUR (horloge du pair en avance) passe : l'authenticité
/// est déjà prouvée par la signature, et clamper là créerait des faux négatifs
/// sur les appareils à l'heure décalée (iPhone vu à +8 min).
fn rendezvous_entry_fresh(addr: &tom_dht::DhtNodeAddr, now_ms: u64) -> bool {
    now_ms <= addr.timestamp.saturating_add(RENDEZVOUS_MAX_AGE_MS)
}

/// Publish ourselves into the shared DHT rendezvous and inject any peers found.
///
/// Runs off-loop (spawned) so DHT latency never blocks the runtime. Discovered
/// peers are fed back as `DhtLookupResult` commands → the existing handler dials
/// them and joins gossip. This is what lets a node with ZERO prior knowledge
/// (e.g. a phone on cellular that lost its only peer) find the live network with
/// no bootstrap peer, no relay, and no privileged node.
#[allow(clippy::too_many_arguments)]
fn spawn_rendezvous_round(
    node: &TomNode,
    local_id: &NodeId,
    dht_handle: Option<&tom_dht::AsyncDht>,
    cmd_tx: &mpsc::Sender<RuntimeCommand>,
    ts_floor: &mut u64,
    secret_seed: &[u8; 32],
    username: &str,
    app_build: u32,
    discovery_start: std::time::Instant,
    event_tx: mpsc::Sender<ProtocolEvent>,
    is_startup: bool,
) {
    let Some(dht) = dht_handle.cloned() else {
        return;
    };
    let self_addr = build_self_dht_addr(node, local_id, ts_floor, secret_seed, username, app_build);
    let own_id = self_addr.node_id.clone();
    let tx = cmd_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = tom_dht::rendezvous_publish(&dht, &self_addr).await {
            tracing::debug!("rendezvous publish failed: {e}");
        }
        let found = tom_dht::rendezvous_discover(&dht, &own_id).await;
        // SECURITY: only inject entries with a valid proof-of-possession signature.
        // Drops squatted/poisoned slots (forged node_id or addrs) PUIS les
        // entrées périmées (fantômes : nœuds morts qui ne se réannoncent plus).
        // Sans le filtre de fraîcheur, un nœud arrêté restait injecté et redialé
        // en boucle jusqu'à expiration DHT (~2 h) — pollution de la liste de
        // pairs (cf. node de test 38ceabfe vu ~6 min après sa mort).
        let total = found.len();
        let now = now_ms();
        let peers: Vec<_> = found
            .into_iter()
            .filter(rendezvous_entry_authentic)
            .filter(|a| rendezvous_entry_fresh(a, now))
            .collect();
        let rejected = total - peers.len();
        let elapsed = discovery_start.elapsed().as_millis() as u64;
        let phase_label = if is_startup { "startup" } else { "tick" };
        if !peers.is_empty() || rejected > 0 {
            tracing::info!(
                injectés = peers.len(),
                rejetés_sig = rejected,
                "⏱️ DISCO t+{elapsed}ms : rendezvous discover ({phase_label}) returned {total} — keeping {} peers",
                peers.len()
            );
            let _ = event_tx
                .send(ProtocolEvent::DiscoveryTiming {
                    elapsed_ms: elapsed,
                    detail: format!(
                        "Rendez-vous ({}) : {} pairs gardés, {} rejetés (signature)",
                        if phase_label == "startup" { "démarrage" } else { "cycle" },
                        peers.len(),
                        rejected
                    ),
                })
                .await;
        }
        for addr in peers {
            let _ = tx.send(RuntimeCommand::DhtLookupResult { addr }).await;
        }
    });
}

async fn bootstrap_join_peer(
    node: &TomNode,
    gossip_sender: Option<&GossipSender>,
    endpoint_addr: tom_connect::EndpointAddr,
    source: BootstrapSource,
    bootstrap_phase: &mut BootstrapPhase,
    discovery_start: std::time::Instant,
    event_tx: mpsc::Sender<ProtocolEvent>,
) {
    let endpoint_id = endpoint_addr.id;
    let node_id = NodeId::from_endpoint_id(endpoint_id);
    let elapsed = discovery_start.elapsed().as_millis() as u64;
    let source_name = match source {
        BootstrapSource::Manual => "manual",
        BootstrapSource::Mdns => "mdns",
        BootstrapSource::Dht => "dht",
        BootstrapSource::PeerPresent => "peer_present",
    };
    tracing::info!(
        peer = %node_id,
        source = %source,
        phase = ?bootstrap_phase,
        "⏱️ DISCO t+{elapsed}ms : join peer {} via {}",
        node_id,
        source_name
    );
    let _ = event_tx
        .send(ProtocolEvent::DiscoveryTiming {
            elapsed_ms: elapsed,
            detail: format!("Connexion au pair {} (source {})", node_id, source_name),
        })
        .await;

    // INVARIANT: add_peer_addr() BEFORE join_peers()
    // so MemoryLookup has the address when gossip dials.
    node.add_peer_addr(endpoint_addr).await;

    // Le pool de transport (ALPN transport/0, celui qui alimente
    // pairs_connectes) ne dial que paresseusement, au premier send()
    // applicatif — contrairement au gossip, proactif. Sans ce chauffage,
    // un pair fraîchement découvert reste invisible dans pairs_connectes
    // jusqu'au prochain heartbeat (jusqu'à HEARTBEAT_INTERVAL_MS plus tard).
    // Détaché : un pair injoignable ne doit jamais bloquer la boucle de
    // découverte (même principe que le publish DHT détaché, #36).
    let transport_sender = node.sender();
    tokio::spawn(async move {
        if let Err(error) = transport_sender.ensure_connected(node_id).await {
            tracing::debug!(peer = %node_id, %error, "warm transport dial failed — will retry lazily on next send");
        }
    });

    if let Some(sender) = gossip_sender {
        if let Err(error) = sender.join_peers(vec![endpoint_id]).await {
            tracing::warn!(peer = %node_id, source = %source, %error, "bootstrap: gossip join failed — dial/join will not be attempted");
        }
    } else {
        // Gossip subscription failed at startup — cannot attempt join_peers, so peer won't be contacted
        tracing::warn!(peer = %node_id, source = %source, "bootstrap: gossip unavailable — peer hint accepted but join_peers not possible");
    }

    if *bootstrap_phase != BootstrapPhase::Converged {
        let previous_phase = *bootstrap_phase;
        bootstrap_phase.on_hint_accepted();
        tracing::info!(
            from = ?previous_phase,
            to = ?bootstrap_phase,
            peer = %node_id,
            source = %source,
            "bootstrap: phase advanced"
        );
    }
}

#[cfg(test)]
mod tests {
    use super::{
        liveness_is_stale, rendezvous_entry_authentic, rendezvous_entry_fresh, NodeId,
        LIVENESS_STALE_MS, RENDEZVOUS_MAX_AGE_MS,
    };
    use ed25519_dalek::{Signer, SigningKey};
    use rand::SeedableRng;

    fn signed_rendezvous_addr(seed_u64: u64) -> (tom_dht::DhtNodeAddr, [u8; 32]) {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed_u64);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        let seed = secret.to_bytes();
        let node_id = secret.public().to_string();
        let mut addr = tom_dht::DhtNodeAddr {
            node_id,
            relay_urls: vec!["http://1.2.3.4:3340".into()],
            direct_addrs: vec!["1.2.3.4:43925".into()],
            username: String::new(),
            app_build: 0,
            timestamp: 1_000_000,
            sig: Vec::new(),
        };
        let sig = SigningKey::from_bytes(&seed).sign(&addr.signing_bytes());
        addr.sig = sig.to_bytes().to_vec();
        (addr, seed)
    }

    #[test]
    fn rendezvous_fresh_entry_kept_stale_rejected() {
        let (mut addr, _) = signed_rendezvous_addr(1);
        // Entrée réannoncée « il y a 30s » → fraîche.
        let now = 100_000_000u64;
        addr.timestamp = now - 30_000;
        assert!(rendezvous_entry_fresh(&addr, now), "30s doit être frais");
        // Entrée d'un nœud mort il y a 20 min → périmée (fantôme).
        addr.timestamp = now - 20 * 60 * 1000;
        assert!(
            !rendezvous_entry_fresh(&addr, now),
            "20 min doit être rejeté (fantôme)"
        );
        // Juste au bord (10 min pile) → encore accepté.
        addr.timestamp = now - RENDEZVOUS_MAX_AGE_MS;
        assert!(rendezvous_entry_fresh(&addr, now), "10 min pile = limite incluse");
        // Timestamp FUTUR (horloge du pair en avance de 8 min) → accepté
        // (tolérance décalage d'horloge, l'authenticité est signée).
        addr.timestamp = now + 8 * 60 * 1000;
        assert!(
            rendezvous_entry_fresh(&addr, now),
            "timestamp futur (skew) doit passer"
        );
    }

    #[test]
    fn rendezvous_valid_signature_accepted() {
        let (addr, _) = signed_rendezvous_addr(1);
        assert!(rendezvous_entry_authentic(&addr));
    }

    #[test]
    fn rendezvous_unsigned_rejected() {
        let (mut addr, _) = signed_rendezvous_addr(2);
        addr.sig.clear();
        assert!(!rendezvous_entry_authentic(&addr), "entrée non signée doit être rejetée");
    }

    #[test]
    fn rendezvous_tampered_addr_rejected() {
        let (mut addr, _) = signed_rendezvous_addr(3);
        addr.direct_addrs = vec!["6.6.6.6:3340".into()]; // attacker swaps the address
        assert!(!rendezvous_entry_authentic(&addr), "addr falsifiée doit être rejetée");
    }

    #[test]
    fn dht_addr_borne_le_nombre_dadresses() {
        use super::{dht_addr_to_endpoint_addr, MAX_DHT_ADDRS};
        // Enregistrement DHT malveillant : des centaines d'adresses distinctes.
        let (mut addr, _) = signed_rendezvous_addr(42);
        addr.direct_addrs = (0..200u16)
            .map(|i| format!("10.0.{}.1:3340", i % 256))
            .collect();
        addr.relay_urls = (0..200u16)
            .map(|i| format!("http://relay-{i}.example:3340"))
            .collect();
        let ep = dht_addr_to_endpoint_addr(&addr).expect("node_id valide");
        // Au plus MAX_DHT_ADDRS relais + MAX_DHT_ADDRS directs traités (anti-DoS).
        assert!(
            ep.addrs.len() <= 2 * MAX_DHT_ADDRS,
            "les adresses DHT doivent être bornées, obtenu {}",
            ep.addrs.len()
        );
    }

    fn test_node_id(seed_u64: u64) -> NodeId {
        let mut rng = rand::rngs::StdRng::seed_from_u64(seed_u64);
        let secret = tom_connect::SecretKey::generate(&mut rng);
        secret.public().to_string().parse().unwrap()
    }

    #[test]
    fn join_cooldown_borne_les_fantomes_non_connectes() {
        use super::join_cooldown_ok;
        use std::time::{Duration, Instant};
        let mut cooldown = std::collections::HashMap::new();
        let peer_paths = std::collections::HashMap::new(); // aucun pair connecté
        let phantom = test_node_id(7);
        let t0 = Instant::now();
        // 1er join autorisé (découverte immédiate).
        assert!(join_cooldown_ok(&phantom, &mut cooldown, &peer_paths, t0));
        // Juste après : refusé (cooldown non-connecté 5 s → pas de hammering).
        assert!(!join_cooldown_ok(&phantom, &mut cooldown, &peer_paths, t0 + Duration::from_secs(2)));
        // Après 5 s : re-tenté (reconvergence rapide si le pair est vivant).
        assert!(join_cooldown_ok(&phantom, &mut cooldown, &peer_paths, t0 + Duration::from_secs(6)));
    }

    #[test]
    fn join_cooldown_connecte_fenetre_longue() {
        use super::join_cooldown_ok;
        use std::time::{Duration, Instant};
        let node = test_node_id(9);
        let mut cooldown = std::collections::HashMap::new();
        let mut peer_paths = std::collections::HashMap::new();
        peer_paths.insert(node, tom_transport::PathKind::Direct); // connecté
        let t0 = Instant::now();
        assert!(join_cooldown_ok(&node, &mut cooldown, &peer_paths, t0));
        // Connecté : fenêtre 60 s → encore refusé à 30 s (anti-rafale).
        assert!(!join_cooldown_ok(&node, &mut cooldown, &peer_paths, t0 + Duration::from_secs(30)));
        // Après 60 s : autorisé.
        assert!(join_cooldown_ok(&node, &mut cooldown, &peer_paths, t0 + Duration::from_secs(61)));
    }

    #[test]
    fn rendezvous_forged_node_id_rejected() {
        // Attacker keeps a valid signature but swaps node_id to impersonate someone.
        let (mut addr, _) = signed_rendezvous_addr(4);
        let (other, _) = signed_rendezvous_addr(5);
        addr.node_id = other.node_id;
        assert!(!rendezvous_entry_authentic(&addr), "node_id usurpé doit être rejeté");
    }

    #[test]
    fn rendezvous_garbage_sig_rejected() {
        let (mut addr, _) = signed_rendezvous_addr(6);
        addr.sig = vec![0u8; 64];
        assert!(!rendezvous_entry_authentic(&addr));
    }

    #[test]
    fn direct_addr_dialability() {
        use super::direct_addr_is_dialable;
        use std::net::IpAddr;
        // Dialable: public + private LAN (needed for same-LAN DHT discovery).
        assert!(direct_addr_is_dialable("1.2.3.4".parse::<IpAddr>().unwrap()));
        assert!(direct_addr_is_dialable("192.168.0.83".parse::<IpAddr>().unwrap()));
        assert!(direct_addr_is_dialable("10.0.0.1".parse::<IpAddr>().unwrap()));
        // Never-dialable: dropped.
        assert!(!direct_addr_is_dialable("127.0.0.1".parse::<IpAddr>().unwrap()));
        assert!(!direct_addr_is_dialable("0.0.0.0".parse::<IpAddr>().unwrap()));
        assert!(!direct_addr_is_dialable("169.254.1.1".parse::<IpAddr>().unwrap()));
        assert!(!direct_addr_is_dialable("::1".parse::<IpAddr>().unwrap()));
        assert!(!direct_addr_is_dialable("fe80::1".parse::<IpAddr>().unwrap()));
    }

    #[test]
    fn liveness_fresh_within_window() {
        let now = 1_000_000u64;
        assert!(!liveness_is_stale(now, now, LIVENESS_STALE_MS));
        assert!(!liveness_is_stale(now - 1_000, now, LIVENESS_STALE_MS));
        assert!(!liveness_is_stale(now - LIVENESS_STALE_MS, now, LIVENESS_STALE_MS));
    }

    #[test]
    fn liveness_stale_past_window() {
        let now = 1_000_000u64;
        assert!(liveness_is_stale(now - LIVENESS_STALE_MS - 1, now, LIVENESS_STALE_MS));
        assert!(liveness_is_stale(0, now, LIVENESS_STALE_MS));
    }

    #[test]
    fn liveness_future_inbound_never_stale() {
        // Clock skew: last_inbound slightly ahead of now must not underflow to stale.
        let now = 1_000_000u64;
        assert!(!liveness_is_stale(now + 5_000, now, LIVENESS_STALE_MS));
        assert!(!liveness_is_stale(u64::MAX, now, LIVENESS_STALE_MS));
    }

    #[tokio::test(start_paused = true)]
    async fn maintenance_interval_skip_behavior() {
        // Verify that maintenance_interval uses MissedTickBehavior::Skip,
        // not Burst. After a stall (CPU, iOS resume), we want ONE rattrapage
        // tick, not a replay of all missed ticks.
        let mut interval = super::maintenance_interval(std::time::Duration::from_secs(1));

        // Consume the initial tick (standard behavior).
        interval.tick().await;

        // Pause time and advance 10× the period WITHOUT polling.
        // With Burst, this would accumulate 10 missed ticks.
        // With Skip, only 1 tick is available (realigned, next tick ~1s from now).
        tokio::time::advance(std::time::Duration::from_secs(10)).await;

        // Poll once: should succeed immediately (rattrapage tick).
        let ready = tokio::select! {
            _ = interval.tick() => true,
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => false,
        };
        assert!(ready, "après un stall, un tick doit être disponible immédiatement");

        // Poll again: should timeout or require ~1s more (next tick).
        // We're now ~1s away from the next scheduled tick, so a poll should block.
        let ready = tokio::select! {
            _ = interval.tick() => true,
            _ = tokio::time::sleep(std::time::Duration::from_millis(100)) => false,
        };
        assert!(
            !ready,
            "pas de deuxième tick immédiat — Skip réaligne sans replay de backlog"
        );
    }
}
