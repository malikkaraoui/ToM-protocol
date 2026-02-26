/// Full-protocol responder — runs ProtocolRuntime with auto-response behavior.
///
/// - Echoes chat messages: `ECHO:<payload>`
/// - Responds to `PING:<seq>` with `PONG:<seq>`
/// - Responds to `BURST:<seq>` with `BURST-ACK:<seq>`
/// - Auto-accepts group invites
/// - Echoes group messages: `GROUP-ECHO:<text>`
/// - Runs indefinitely until Ctrl+C
use std::time::Instant;

use tom_protocol::{ProtocolEvent, ProtocolRuntime, RuntimeConfig};
use tom_transport::{TomNode, TomNodeConfig};

pub struct ResponderConfig {
    pub name: String,
    pub max_message_size: usize,
}

pub async fn run(config: ResponderConfig) -> anyhow::Result<()> {
    let start = Instant::now();
    let running = crate::common::setup_ctrlc();

    let node_config = TomNodeConfig::new().max_message_size(config.max_message_size);
    let node = TomNode::bind(node_config).await?;

    eprintln!("Responder Node ID: {}", node.id());
    eprintln!("Name: {}", config.name);
    eprintln!("Waiting for connections...\n");

    let runtime_config = RuntimeConfig {
        username: config.name.clone(),
        encryption: true,
        ..Default::default()
    };

    let mut channels = ProtocolRuntime::spawn(node, runtime_config);
    let handle = channels.handle.clone();

    let mut msg_count: u64 = 0;
    let mut group_msg_count: u64 = 0;

    while running.load(std::sync::atomic::Ordering::Relaxed) {
        tokio::select! {
            // ── Chat messages ────────────────────────────────────────
            msg = channels.messages.recv() => {
                let Some(msg) = msg else { break };
                msg_count += 1;
                let text = String::from_utf8_lossy(&msg.payload);

                if msg_count % 100 == 1 || msg_count <= 10 {
                    eprintln!(
                        "[{:>7.1}s] msg #{msg_count} from {} | enc={} sig={} | {:?}",
                        start.elapsed().as_secs_f64(),
                        short_id(&msg.from.to_string()),
                        msg.was_encrypted,
                        msg.signature_valid,
                        truncate(&text, 60),
                    );
                }

                let reply = build_reply(&text);
                if let Err(e) = handle.send_message(msg.from, reply.into_bytes()).await {
                    eprintln!("  reply failed: {e}");
                }
            }

            // ── Protocol events ──────────────────────────────────────
            evt = channels.events.recv() => {
                let Some(evt) = evt else { break };
                match &evt {
                    ProtocolEvent::PeerDiscovered { node_id, username, source } => {
                        eprintln!("[{:>7.1}s] Peer discovered: {} \"{}\" ({:?})", start.elapsed().as_secs_f64(), short_id(&node_id.to_string()), username, source);
                    }
                    ProtocolEvent::PeerStale { node_id } => {
                        eprintln!("[{:>7.1}s] Peer stale: {}", start.elapsed().as_secs_f64(), short_id(&node_id.to_string()));
                    }
                    ProtocolEvent::PeerOnline { node_id } => {
                        eprintln!("[{:>7.1}s] Peer online: {}", start.elapsed().as_secs_f64(), short_id(&node_id.to_string()));
                    }
                    ProtocolEvent::GossipNeighborUp { node_id } => {
                        eprintln!("[{:>7.1}s] Neighbor UP: {}", start.elapsed().as_secs_f64(), short_id(&node_id.to_string()));
                    }
                    ProtocolEvent::GossipNeighborDown { node_id } => {
                        eprintln!("[{:>7.1}s] Neighbor DOWN: {}", start.elapsed().as_secs_f64(), short_id(&node_id.to_string()));
                    }
                    ProtocolEvent::GroupInviteReceived { invite } => {
                        eprintln!("[{:>7.1}s] Group invite: {} — auto-accepting", start.elapsed().as_secs_f64(), invite.group_name);
                        if let Err(e) = handle.accept_invite(invite.group_id.clone()).await {
                            eprintln!("  accept failed: {e}");
                        }
                    }
                    ProtocolEvent::GroupJoined { group_id, group_name } => {
                        eprintln!("[{:>7.1}s] Joined group: {} ({})", start.elapsed().as_secs_f64(), group_name, group_id);
                    }
                    ProtocolEvent::GroupMessageReceived { message } => {
                        group_msg_count += 1;
                        let text = if message.encrypted {
                            format!("[encrypted] {}", message.text)
                        } else {
                            message.text.clone()
                        };

                        if group_msg_count % 100 == 1 || group_msg_count <= 10 {
                            eprintln!(
                                "[{:>7.1}s] group msg #{group_msg_count} from {} | {:?}",
                                start.elapsed().as_secs_f64(),
                                short_id(&message.sender_id.to_string()),
                                truncate(&text, 60),
                            );
                        }

                        // Echo back to the group
                        let reply = format!("GROUP-ECHO:{}", message.text);
                        if let Err(e) = handle.send_group_message(message.group_id.clone(), reply).await {
                            eprintln!("  group reply failed: {e}");
                        }
                    }
                    ProtocolEvent::GroupShadowPromoted { group_id, new_hub_id } => {
                        eprintln!("[{:>7.1}s] SHADOW PROMOTED for {} (new hub: {})", start.elapsed().as_secs_f64(), group_id, short_id(&new_hub_id.to_string()));
                    }
                    ProtocolEvent::PathChanged { event } => {
                        eprintln!("[{:>7.1}s] Path changed: {:?}", start.elapsed().as_secs_f64(), event);
                    }
                    ProtocolEvent::Error { description } => {
                        eprintln!("[{:>7.1}s] ERROR: {description}", start.elapsed().as_secs_f64());
                    }
                    _ => {}
                }
            }

            // ── Status changes ───────────────────────────────────────
            _sc = channels.status_changes.recv() => {
                // Silently consume
            }

            // ── Shutdown check ───────────────────────────────────────
            _ = tokio::time::sleep(std::time::Duration::from_millis(500)) => {}
        }
    }

    eprintln!("\nShutting down responder...");
    eprintln!("  Chat messages echoed: {msg_count}");
    eprintln!("  Group messages echoed: {group_msg_count}");
    handle.shutdown().await;
    tokio::time::sleep(std::time::Duration::from_millis(200)).await;

    Ok(())
}

/// Build the reply based on the incoming message content.
fn build_reply(text: &str) -> String {
    if let Some(seq) = text.strip_prefix("PING:") {
        format!("PONG:{seq}")
    } else if let Some(seq) = text.strip_prefix("BURST:") {
        format!("BURST-ACK:{seq}")
    } else {
        format!("ECHO:{text}")
    }
}

fn short_id(id: &str) -> &str {
    if id.len() > 8 { &id[..8] } else { id }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() > max { &s[..max] } else { s }
}
