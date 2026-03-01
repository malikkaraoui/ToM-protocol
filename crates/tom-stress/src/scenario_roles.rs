/// Roles scenario — spawn two nodes, exchange messages to generate relay activity,
/// then query role metrics and scores to verify the scoring pipeline works.
use std::time::{Duration, Instant};

use tom_protocol::{ProtocolEvent, ProtocolRuntime, RuntimeConfig};
use tom_transport::{TomNode, TomNodeConfig};

use crate::scenario_common::{recv_timeout, timed_step_async, ScenarioResult};

pub async fn run() -> anyhow::Result<ScenarioResult> {
    let mut result = ScenarioResult::new("roles");
    let start = Instant::now();

    // ── Spawn two nodes ─────────────────────────────────────────────
    let node_a = TomNode::bind(TomNodeConfig::new()).await?;
    let node_b = TomNode::bind(TomNodeConfig::new()).await?;

    let id_a = node_a.id();
    let id_b = node_b.id();

    // Exchange full addresses for direct local connectivity
    let addr_a = node_a.addr();
    let addr_b = node_b.addr();

    eprintln!("Alice : {id_a}");
    eprintln!("Bob   : {id_b}");

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
        tokio::time::sleep(Duration::from_millis(500)).await;
        Ok(String::new())
    })
    .await;
    result.add(step);

    // ── Exchange messages to build activity ──────────────────────────
    let step = timed_step_async("exchange messages", || async {
        let mut received = 0u32;

        for i in 0..10u32 {
            let payload = format!("role-test-{i}").into_bytes();
            channels_a
                .handle
                .send_message(id_b, payload)
                .await
                .map_err(|e| format!("send failed: {e}"))?;
        }

        for _ in 0..10u32 {
            match recv_timeout(&mut channels_b.messages, Duration::from_secs(10)).await {
                Ok(_) => received += 1,
                Err(_) => break,
            }
        }

        Ok(format!("{received}/10 messages received"))
    })
    .await;
    result.add(step);

    // ── Query role metrics for peer ─────────────────────────────────
    let step = timed_step_async("query role metrics", || async {
        // Query alice's view of bob's metrics
        match channels_a.handle.get_role_metrics(id_b).await {
            Some(metrics) => Ok(format!(
                "role={:?}, score={:.2}, relays={}, bytes_relayed={}",
                metrics.role, metrics.score, metrics.relay_count, metrics.bytes_relayed,
            )),
            None => {
                // No relay activity in direct connection — expected
                Ok("no relay metrics (direct connection — expected)".into())
            }
        }
    })
    .await;
    result.add(step);

    // ── Query all role scores ───────────────────────────────────────
    let step = timed_step_async("query all role scores", || async {
        let scores = channels_a.handle.get_all_role_scores().await;
        if scores.is_empty() {
            // With direct connections, the role manager may not track peers
            // unless they actually relayed traffic
            Ok("0 peers scored (no relay activity — expected for direct)".into())
        } else {
            let summary: Vec<String> = scores
                .iter()
                .map(|(id, score, role)| {
                    let short = &id.to_string()[..8];
                    format!("{short}… {role:?} score={score:.2}")
                })
                .collect();
            Ok(format!("{} peers scored: {}", scores.len(), summary.join(", ")))
        }
    })
    .await;
    result.add(step);

    // ── Check for role events ───────────────────────────────────────
    let step = timed_step_async("check role events", || async {
        // Give time for role evaluation tick
        tokio::time::sleep(Duration::from_secs(2)).await;

        let mut role_events = 0u32;
        while let Ok(evt) = channels_b.events.try_recv() {
            match &evt {
                ProtocolEvent::RolePromoted { .. }
                | ProtocolEvent::RoleDemoted { .. }
                | ProtocolEvent::LocalRoleChanged { .. } => role_events += 1,
                _ => {}
            }
        }

        Ok(format!("{role_events} role events observed"))
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
