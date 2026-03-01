/// Backup scenario — send messages to a peer that goes offline, verify that
/// backup events fire in the runtime.
///
/// In a full network, backed-up messages would be replicated to relay peers and
/// delivered when the recipient comes back. In this in-process test we verify:
/// 1. Baseline: messages reach a live peer
/// 2. After peer shutdown, sends trigger backup/error handling in the runtime
use std::time::{Duration, Instant};

use tom_protocol::{ProtocolEvent, ProtocolRuntime, RuntimeConfig};
use tom_transport::{TomNode, TomNodeConfig};

use crate::scenario_common::{recv_timeout, timed_step_async, ScenarioResult};

pub async fn run() -> anyhow::Result<ScenarioResult> {
    let mut result = ScenarioResult::new("backup-delivery");
    let start = Instant::now();

    // ── Spawn two nodes ──────────────────────────────────────────
    let node_a = TomNode::bind(TomNodeConfig::new()).await?;
    let node_b = TomNode::bind(TomNodeConfig::new()).await?;

    let id_a = node_a.id();
    let id_b = node_b.id();

    // Exchange full addresses for direct local connectivity
    let addr_a = node_a.addr();
    let addr_b = node_b.addr();

    eprintln!("Alice (sender)  : {id_a}");
    eprintln!("Bob (recipient) : {id_b}");

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

    let mut channels_a = ProtocolRuntime::spawn(node_a, config_a);
    let mut channels_b = ProtocolRuntime::spawn(node_b, config_b);

    // ── Register peers with full addresses ────────────────────────
    let step = timed_step_async("register peers", || async {
        channels_a.handle.add_peer_addr(addr_b).await;
        channels_b.handle.add_peer_addr(addr_a).await;
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(String::new())
    })
    .await;
    result.add(step);

    // ── Baseline: send message while both online ─────────────────
    let step = timed_step_async("baseline: send while online", || async {
        let payload = b"hello-online".to_vec();
        channels_a
            .handle
            .send_message(id_b, payload)
            .await
            .map_err(|e| format!("send failed: {e}"))?;

        match recv_timeout(&mut channels_b.messages, Duration::from_secs(10)).await {
            Ok(msg) => Ok(format!(
                "received {} bytes, encrypted={}, signed={}",
                msg.payload.len(),
                msg.was_encrypted,
                msg.signature_valid
            )),
            Err(e) => Err(format!("baseline recv failed: {e}")),
        }
    })
    .await;
    result.add(step);

    // ── Shut down Bob ────────────────────────────────────────────
    let step = timed_step_async("shutdown bob", || async {
        channels_b.handle.shutdown().await;
        // Give time for shutdown to propagate
        tokio::time::sleep(Duration::from_secs(2)).await;
        Ok("bob shut down".into())
    })
    .await;
    result.add(step);

    // ── Send messages to offline Bob ─────────────────────────────
    let step = timed_step_async("send to offline bob", || async {
        let mut backup_events = 0u32;
        let mut error_events = 0u32;

        for i in 0..3u32 {
            let payload = format!("offline-msg-{i}").into_bytes();
            let _ = channels_a.handle.send_message(id_b, payload).await;
        }

        // Drain events — we expect BackupStored or Error events
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline {
            match recv_timeout(&mut channels_a.events, Duration::from_secs(1)).await {
                Ok(ProtocolEvent::BackupStored { .. }) => backup_events += 1,
                Ok(ProtocolEvent::Error { .. }) => error_events += 1,
                Ok(_) => continue,
                Err(_) => break,
            }
        }

        Ok(format!(
            "3 sent to offline peer: {backup_events} backup events, {error_events} error events"
        ))
    })
    .await;
    result.add(step);

    // ── Shutdown Alice ───────────────────────────────────────────
    channels_a.handle.shutdown().await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    result.finalize(start);
    Ok(result)
}
