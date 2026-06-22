/// The protocol runtime event loop — thin orchestrator.
///
/// Owns RuntimeState + TomNode. Multiplexes over transport events,
/// application commands, and timers. Delegates all logic to RuntimeState,
/// executes effects via executor.
use tokio::sync::{broadcast, mpsc};
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
    let mut cache_cleanup = tokio::time::interval(state.config.cache_cleanup_interval);
    let mut tracker_cleanup = tokio::time::interval(state.config.tracker_cleanup_interval);
    let mut heartbeat_check = tokio::time::interval(state.config.heartbeat_interval);
    let mut group_hub_heartbeat = tokio::time::interval(state.config.group_hub_heartbeat_interval);
    let mut backup_tick = tokio::time::interval(state.config.backup_tick_interval);
    let mut gossip_announce = tokio::time::interval(state.config.gossip_announce_interval);
    let mut shadow_ping = tokio::time::interval(state.config.shadow_ping_interval);
    let mut subnet_eval = tokio::time::interval(std::time::Duration::from_secs(30));
    let mut role_eval = tokio::time::interval(std::time::Duration::from_secs(300));
    let mut state_save = tokio::time::interval(std::time::Duration::from_secs(30));
    let mut dht_republish = tokio::time::interval(std::time::Duration::from_secs(30 * 60));
    let mut delivery_deadline = tokio::time::interval(std::time::Duration::from_secs(5));
    let mut hub_cleanup = tokio::time::interval(std::time::Duration::from_secs(60));
    let mut reconnect_check = tokio::time::interval(std::time::Duration::from_secs(15));
    // Shared DHT rendezvous: periodic re-announce + zero-config peer discovery.
    let mut rendezvous_tick = tokio::time::interval(std::time::Duration::from_secs(60));

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

    // ── DHT setup ──────────────────────────────────────────────────
    let secret_seed = node.secret_key_seed();
    // Clone the async DHT handle for spawned lookup tasks (cheap Arc clone)
    let dht_handle: Option<tom_dht::AsyncDht> =
        state.dht().map(|d| d.async_dht());

    // Publish to DHT at startup (BEP-0044)
    {
        let (relay_urls, direct_addrs) = extract_node_addrs(&node);
        state.publish_to_dht(&secret_seed, relay_urls, direct_addrs).await;
    }

    // Monotonic floor for rendezvous publication timestamps (see build_self_dht_addr).
    let mut rendezvous_ts_floor: u64 = 0;

    // Announce into the shared rendezvous + pull live peers immediately, so a
    // node with no prior knowledge starts finding the network from the first tick.
    spawn_rendezvous_round(
        &node,
        &state.local_id,
        dht_handle.as_ref(),
        &cmd_tx,
        &mut rendezvous_ts_floor,
    );

    // ── PeerPresent receiver from relay ────────────────────────────────
    let mut peer_present_rx = node.take_peer_present_rx();
    let mut bootstrap_hint_rx = node.take_bootstrap_hint_rx();
    let mut bootstrap_phase = BootstrapPhase::LanProbe;

    // ── Embedded relay service ─────────────────────────────────────────
    let mut embedded_relay = super::EmbeddedRelayService::new();

    // Auto-start embedded relay if enabled in config
    if state.config.enable_embedded_relay {
        let relay_config = super::EmbeddedRelayConfig {
            bind_addr: state.config.embedded_relay_bind_addr,
            advertise_addr: state.config.embedded_relay_advertise_addr,
        };
        let startup_effects = match embedded_relay.start(relay_config).await {
            Ok(url) => {
                node.reprobe_relays().await;
                state.handle_command(super::RuntimeCommand::EmbeddedRelayStarted { url })
            }
            Err(error) => state.handle_command(super::RuntimeCommand::EmbeddedRelayFailed { error }),
        };
        execute_effects(startup_effects, &node, &msg_tx, &status_tx, &event_tx, &metrics).await;
    }

    // ── Rejoin groups after restart (one-shot) ────────────────────────
    let rejoin_effects = state.build_rejoin_effects();
    if !rejoin_effects.is_empty() {
        execute_effects(rejoin_effects, &node, &msg_tx, &status_tx, &event_tx, &metrics).await;
    }

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
                        // Build EndpointAddr from DHT record and inject into transport
                        if let Some(endpoint_addr) = dht_addr_to_endpoint_addr(addr) {
                            bootstrap_join_peer(
                                &node,
                                gossip_sender.as_ref(),
                                endpoint_addr,
                                BootstrapSource::Dht,
                                &mut bootstrap_phase,
                            ).await;
                        }
                        state.handle_command(cmd)
                    }
                    RuntimeCommand::StartEmbeddedRelay { config } => {
                        match embedded_relay.start(config).await {
                            Ok(url) => {
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
                        state.handle_command(RuntimeCommand::EmbeddedRelayStopped)
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
                vec![RuntimeEffect::Emit(ProtocolEvent::PathChanged { event })]
            }

            // ── 3a. LAN bootstrap hints (mDNS) ───────────────
            event = async {
                match bootstrap_hint_rx.as_mut() {
                    Some(rx) => rx.recv().await,
                    None => std::future::pending().await,
                }
            } => {
                if let Some(BootstrapHint::MdnsDiscovered { endpoint_addr }) = event {
                    let node_id = NodeId::from_endpoint_id(endpoint_addr.id);
                    bootstrap_join_peer(
                        &node,
                        gossip_sender.as_ref(),
                        endpoint_addr,
                        BootstrapSource::Mdns,
                        &mut bootstrap_phase,
                    ).await;
                    state.handle_command(RuntimeCommand::AddPeer { node_id, source: DiscoverySource::Mdns })
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
                if let Some((endpoint_id, relay_url)) = event {
                    let node_id = NodeId::from_endpoint_id(endpoint_id);
                    let addr = tom_connect::EndpointAddr::new(endpoint_id).with_relay_url(relay_url);
                    bootstrap_join_peer(
                        &node,
                        gossip_sender.as_ref(),
                        addr,
                        BootstrapSource::PeerPresent,
                        &mut bootstrap_phase,
                    ).await;
                    state.handle_command(RuntimeCommand::AddPeer { node_id, source: DiscoverySource::PeerPresent })
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

            // ── 9. Gossip events ────────────────────────────────
            event = async {
                match gossip_receiver.as_mut() {
                    Some(rx) => rx.next().await,
                    None => std::future::pending::<Option<_>>().await,
                }
            } => {
                if let Some(Ok(event)) = event {
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
                                tracing::info!(
                                    peer = %node_id,
                                    "bootstrap: converged via GossipNeighborUp"
                                );
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

                            // If we lost all gossip neighbors, re-bootstrap:
                            // rejoin known peers so we reconnect instead of staying isolated.
                            if let Some(ref sender) = gossip_sender {
                                let known_peers: Vec<_> = state.topology.peers()
                                    .map(|p| *p.node_id.as_endpoint_id())
                                    .collect();
                                if !known_peers.is_empty() {
                                    tracing::info!(
                                        peers = known_peers.len(),
                                        "neighbor down — rejoining {} known peers",
                                        known_peers.len()
                                    );
                                    let _ = sender.join_peers(known_peers).await;
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

            // ── 15b. Timer: DHT rendezvous (60s) — zero-config discovery ──
            _ = rendezvous_tick.tick() => {
                spawn_rendezvous_round(
                    &node,
                    &state.local_id,
                    dht_handle.as_ref(),
                    &cmd_tx,
                    &mut rendezvous_ts_floor,
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
                if let Some(ref sender) = gossip_sender {
                    let known_node_ids: Vec<_> =
                        state.topology.peers().map(|p| p.node_id).collect();
                    let known_eids: Vec<_> =
                        known_node_ids.iter().map(|n| *n.as_endpoint_id()).collect::<Vec<_>>();

                    // Isolation: no live QUIC connection means we lost the network.
                    let connected = node.connected_peers().await;
                    if connected.is_empty() {
                        if bootstrap_phase.on_isolated() {
                            tracing::info!(
                                "reconnect_check: isolé (0 connexion) — retour en amorçage, redécouverte active"
                            );
                        }
                        // Re-announce ourselves and re-resolve known peers via DHT to
                        // pick up fresh addresses (old ones may be dead after a change).
                        let (relay_urls, direct_addrs) = extract_node_addrs(&node);
                        state.publish_to_dht(&secret_seed, relay_urls, direct_addrs).await;
                        // Zero-config recovery: hit the shared rendezvous NOW (don't wait
                        // for the 60s tick) to find live peers we never heard of.
                        spawn_rendezvous_round(
                            &node,
                            &state.local_id,
                            dht_handle.as_ref(),
                            &cmd_tx,
                            &mut rendezvous_ts_floor,
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
                        let _ = sender.join_peers(known_eids).await;
                    }
                    // Always reprobe relays: triggers PeerPresent even when topology has
                    // known-but-offline peers (e.g. after relay cut or network change).
                    node.reprobe_relays().await;
                }
                Vec::new()
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
                            node.reprobe_relays().await;
                            state.handle_command(RuntimeCommand::EmbeddedRelayStarted { url })
                        }
                        Err(error) => state.handle_command(RuntimeCommand::EmbeddedRelayFailed { error }),
                    };
                    regular_effects.extend(feedback_effects);
                }
                RuntimeEffect::StopEmbeddedRelay => {
                    embedded_relay.stop().await;
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
        execute_effects(regular_effects, &node, &msg_tx, &status_tx, &event_tx, &metrics).await;

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

    // Graceful shutdown
    if let Err(e) = node.shutdown().await {
        tracing::warn!("runtime shutdown error: {e}");
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

/// Convert a DHT node address to an EndpointAddr for transport injection.
fn dht_addr_to_endpoint_addr(addr: &tom_dht::DhtNodeAddr) -> Option<tom_connect::EndpointAddr> {
    let node_id: NodeId = addr.node_id.parse().ok()?;
    let mut addrs = std::collections::BTreeSet::new();

    for url_str in &addr.relay_urls {
        if let Ok(url) = url_str.parse::<tom_connect::RelayUrl>() {
            addrs.insert(TransportAddr::Relay(url));
        }
    }
    for addr_str in &addr.direct_addrs {
        if let Ok(sa) = addr_str.parse::<std::net::SocketAddr>() {
            addrs.insert(TransportAddr::Ip(sa));
        }
    }

    Some(tom_connect::EndpointAddr {
        id: *node_id.as_endpoint_id(),
        addrs,
    })
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
) -> tom_dht::DhtNodeAddr {
    let (relay_urls, direct_addrs) = extract_node_addrs(node);
    let ts = now_ms().max(ts_floor.saturating_add(1));
    *ts_floor = ts;
    tom_dht::DhtNodeAddr {
        node_id: local_id.to_string(),
        relay_urls,
        direct_addrs,
        timestamp: ts,
    }
}

/// Publish ourselves into the shared DHT rendezvous and inject any peers found.
///
/// Runs off-loop (spawned) so DHT latency never blocks the runtime. Discovered
/// peers are fed back as `DhtLookupResult` commands → the existing handler dials
/// them and joins gossip. This is what lets a node with ZERO prior knowledge
/// (e.g. a phone on cellular that lost its only peer) find the live network with
/// no bootstrap peer, no relay, and no privileged node.
fn spawn_rendezvous_round(
    node: &TomNode,
    local_id: &NodeId,
    dht_handle: Option<&tom_dht::AsyncDht>,
    cmd_tx: &mpsc::Sender<RuntimeCommand>,
    ts_floor: &mut u64,
) {
    let Some(dht) = dht_handle.cloned() else {
        return;
    };
    let self_addr = build_self_dht_addr(node, local_id, ts_floor);
    let own_id = self_addr.node_id.clone();
    let tx = cmd_tx.clone();
    tokio::spawn(async move {
        if let Err(e) = tom_dht::rendezvous_publish(&dht, &self_addr).await {
            tracing::debug!("rendezvous publish failed: {e}");
        }
        let peers = tom_dht::rendezvous_discover(&dht, &own_id).await;
        if !peers.is_empty() {
            tracing::info!(count = peers.len(), "rendezvous: injecting discovered peers");
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
) {
    let endpoint_id = endpoint_addr.id;
    let node_id = NodeId::from_endpoint_id(endpoint_id);
    tracing::info!(peer = %node_id, source = %source, phase = ?bootstrap_phase, "bootstrap: accepted peer hint");

    // INVARIANT: add_peer_addr() BEFORE join_peers()
    // so MemoryLookup has the address when gossip dials.
    node.add_peer_addr(endpoint_addr).await;

    if let Some(sender) = gossip_sender {
        if let Err(error) = sender.join_peers(vec![endpoint_id]).await {
            tracing::debug!(peer = %node_id, source = %source, %error, "bootstrap: gossip join failed");
        }
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
