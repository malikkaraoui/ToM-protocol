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

use iroh_gossip::Gossip;
use iroh_gossip::api::Event as GossipEvent;
use n0_future::StreamExt;
use tom_transport::PathEvent;

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
    mut cmd_rx: mpsc::Receiver<RuntimeCommand>,
    msg_tx: mpsc::Sender<DeliveredMessage>,
    status_tx: mpsc::Sender<StatusChange>,
    event_tx: mpsc::Sender<ProtocolEvent>,
    mut path_rx: broadcast::Receiver<PathEvent>,
    gossip: Gossip,
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

    // ── Gossip subscription ──────────────────────────────────────────
    let topic_id = iroh_gossip::TopicId::from_bytes(TOM_GOSSIP_TOPIC);
    let bootstrap: Vec<iroh::EndpointId> = gossip_bootstrap_peers
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

    // ── Main loop ────────────────────────────────────────────────────
    loop {
        let effects = tokio::select! {
            // ── 1. Incoming data from transport ─────────────────
            result = node.recv_raw() => {
                match result {
                    Ok((_from, data)) => state.handle_incoming(&data),
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

            else => break,
        };

        // Execute all effects
        execute_effects(effects, &node, &msg_tx, &status_tx, &event_tx).await;
    }

    // Graceful shutdown
    if let Err(e) = node.shutdown().await {
        tracing::warn!("runtime shutdown error: {e}");
    }
}
