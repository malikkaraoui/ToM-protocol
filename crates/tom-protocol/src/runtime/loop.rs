/// The protocol runtime event loop — thin orchestrator.
///
/// Owns RuntimeState + TomNode. Multiplexes over transport events,
/// application commands, and timers. Delegates all logic to RuntimeState,
/// executes effects via executor.
use tokio::sync::{broadcast, mpsc};
use tom_transport::TomNode;

use crate::types::NodeId;

use super::effect::RuntimeEffect;
use super::executor::execute_effects;
use super::state::{GossipInput, RuntimeState};
use super::{DeliveredMessage, ProtocolEvent, RuntimeCommand};
use crate::tracker::StatusChange;

use tom_gossip::Gossip;
use tom_gossip::api::Event as GossipEvent;
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
    let mut role_eval = tokio::time::interval(std::time::Duration::from_secs(60));
    let mut state_save = tokio::time::interval(std::time::Duration::from_secs(30));
    let mut dht_republish = tokio::time::interval(std::time::Duration::from_secs(30 * 60));
    let mut delivery_deadline = tokio::time::interval(std::time::Duration::from_secs(5));

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

    // ── Rejoin groups after restart (one-shot) ────────────────────────
    let rejoin_effects = state.build_rejoin_effects();
    if !rejoin_effects.is_empty() {
        execute_effects(rejoin_effects, &node, &msg_tx, &status_tx, &event_tx, &metrics).await;
    }

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
                        node.add_peer_addr(addr).await;
                        state.handle_command(RuntimeCommand::AddPeer { node_id })
                    }
                    RuntimeCommand::AddPeer { node_id } => {
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
                        state.handle_command(RuntimeCommand::AddPeer { node_id })
                    }
                    RuntimeCommand::DhtLookupResult { ref addr } => {
                        // Build EndpointAddr from DHT record and inject into transport
                        if let Some(endpoint_addr) = dht_addr_to_endpoint_addr(addr) {
                            node.add_peer_addr(endpoint_addr).await;
                        }
                        state.handle_command(cmd)
                    }
                    RuntimeCommand::Shutdown => break,
                    other => state.handle_command(other),
                }
            }

            // ── 3. Path events from transport ───────────────────
            Ok(event) = path_rx.recv() => {
                vec![RuntimeEffect::Emit(ProtocolEvent::PathChanged { event })]
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
                            state.handle_gossip_event(GossipInput::NeighborDown(node_id))
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

            else => break,
        };

        // Intercept BroadcastRoleChange effects (need gossip sender)
        let mut regular_effects = Vec::with_capacity(effects.len());
        for effect in effects {
            if let RuntimeEffect::BroadcastRoleChange(ref announce) = effect {
                if let Some(ref sender) = gossip_sender {
                    if let Ok(bytes) = rmp_serde::to_vec(announce) {
                        if let Err(e) = sender.broadcast(bytes::Bytes::from(bytes)).await {
                            tracing::debug!("gossip: role announce broadcast failed: {e}");
                        }
                    }
                }
            } else {
                regular_effects.push(effect);
            }
        }

        // Execute remaining effects
        execute_effects(regular_effects, &node, &msg_tx, &status_tx, &event_tx, &metrics).await;
    }

    // Save state before shutdown
    state.save_state();

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
