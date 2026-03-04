/// E2E encryption scenario — send encrypted messages between two in-process nodes
/// and verify encryption + signature properties on received messages.
use std::time::{Duration, Instant};

use tom_protocol::{ProtocolRuntime, RuntimeConfig};
use tom_transport::{TomNode, TomNodeConfig};

use crate::scenario_common::{recv_timeout, timed_step_async, ScenarioResult};

/// Number of messages to send in the E2E test.
const MESSAGE_COUNT: u32 = 10;

pub async fn run() -> anyhow::Result<ScenarioResult> {
    let mut result = ScenarioResult::new("e2e-encryption");
    let start = Instant::now();

    // ── Spawn two nodes ─────────────────────────────────────────────
    let node_a = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;
    let node_b = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;

    let id_a = node_a.id();
    let id_b = node_b.id();

    // Exchange full addresses for direct local connectivity
    let addr_a = node_a.addr();
    let addr_b = node_b.addr();

    eprintln!("Node A: {id_a}");
    eprintln!("Node B: {id_b}");

    let config_a = RuntimeConfig {
        username: "alice".into(),
        encryption: true,
        ..Default::default()
    };
    let config_b = RuntimeConfig {
        username: "bob".into(),
        encryption: true,
        ..Default::default()
    };

    let channels_a = ProtocolRuntime::spawn(node_a, config_a);
    let mut channels_b = ProtocolRuntime::spawn(node_b, config_b);

    // ── Register peers with full addresses ────────────────────────
    let step = timed_step_async("register peers", || async {
        channels_a.handle.add_peer_addr(addr_b).await;
        channels_b.handle.add_peer_addr(addr_a).await;

        // Give transport some time to establish paths.
        tokio::time::sleep(Duration::from_millis(750)).await;
        Ok("peer addresses exchanged".into())
    })
    .await;
    result.add(step);

    // ── Send N encrypted messages A → B ─────────────────────────────
    let step = timed_step_async("send encrypted messages", || async {
        for i in 0..MESSAGE_COUNT {
            let payload = format!("encrypted-msg-{i}").into_bytes();
            channels_a
                .handle
                .send_message(id_b, payload)
                .await
                .map_err(|e| format!("send failed: {e}"))?;
            // Stay below low-score anti-spam baseline (~2 msg/s)
            tokio::time::sleep(Duration::from_millis(600)).await;
        }
        Ok(format!("{MESSAGE_COUNT} sent"))
    })
    .await;
    result.add(step);

    // ── Receive and verify ──────────────────────────────────────────
    let step = timed_step_async("receive + verify encryption", || async {
        let mut received = 0u32;
        let mut encrypted_count = 0u32;
        let mut signed_count = 0u32;

        let deadline = Instant::now() + Duration::from_secs(20);
        while received < MESSAGE_COUNT && Instant::now() < deadline {
            match recv_timeout(&mut channels_b.messages, Duration::from_secs(2)).await {
                Ok(msg) => {
                    received += 1;
                    if msg.was_encrypted {
                        encrypted_count += 1;
                    }
                    if msg.signature_valid {
                        signed_count += 1;
                    }
                }
                Err(_) => continue,
            }
        }

        if received == 0 {
            return Err("0 messages received".into());
        }

        if encrypted_count != received {
            return Err(format!(
                "{encrypted_count}/{received} were encrypted (expected all received)"
            ));
        }
        if signed_count != received {
            return Err(format!(
                "{signed_count}/{received} had valid signatures (expected all received)"
            ));
        }

        Ok(format!("{received}/{MESSAGE_COUNT} received, all received encrypted+signed"))
    })
    .await;
    result.add(step);

    // ── Shutdown ────────────────────────────────────────────────────
    channels_a.handle.shutdown().await;
    channels_b.handle.shutdown().await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    result.finalize(start);
    Ok(result)
}
