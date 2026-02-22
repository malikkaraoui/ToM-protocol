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
    let node_a = TomNode::bind(TomNodeConfig::new()).await?;
    let node_b = TomNode::bind(TomNodeConfig::new()).await?;

    let id_a = node_a.id();
    let id_b = node_b.id();

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

    // ── Register peers ──────────────────────────────────────────────
    let step = timed_step_async("register peers", || async {
        channels_a.handle.add_peer(id_b).await;
        channels_b.handle.add_peer(id_a).await;
        // Give iroh time to discover
        tokio::time::sleep(Duration::from_secs(2)).await;
        Ok(String::new())
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

        for _ in 0..MESSAGE_COUNT {
            match recv_timeout(&mut channels_b.messages, Duration::from_secs(10)).await {
                Ok(msg) => {
                    received += 1;
                    if msg.was_encrypted {
                        encrypted_count += 1;
                    }
                    if msg.signature_valid {
                        signed_count += 1;
                    }
                }
                Err(e) => {
                    return Err(format!(
                        "after {received}/{MESSAGE_COUNT}: {e}"
                    ));
                }
            }
        }

        if encrypted_count != MESSAGE_COUNT {
            return Err(format!(
                "{encrypted_count}/{MESSAGE_COUNT} were encrypted (expected all)"
            ));
        }
        if signed_count != MESSAGE_COUNT {
            return Err(format!(
                "{signed_count}/{MESSAGE_COUNT} had valid signatures (expected all)"
            ));
        }

        Ok(format!(
            "{received}/{MESSAGE_COUNT} received, all encrypted+signed"
        ))
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
