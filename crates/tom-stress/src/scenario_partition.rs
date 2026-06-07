/// Partition scenario — split-brain: two isolated node groups verify intra-group
/// messaging succeeds and cross-group messaging is blocked, then heal and verify
/// cross-group messaging resumes.
///
/// Topology (partitioned):
///   Group-A: Alice ─── Bob         (only A↔B peers registered)
///   Group-B: Carol ─── Dave        (only C↔D peers registered)
///
/// Phases:
///   1. Partitioned: intra-group OK, cross-group blocked
///   2. Heal: add cross-group peer addresses
///   3. Post-heal: cross-group delivery confirmed
use std::time::{Duration, Instant};

use tom_protocol::{ProtocolRuntime, RuntimeConfig};
use tom_transport::{TomNode, TomNodeConfig};

use crate::scenario_common::{recv_timeout, timed_step_async, ScenarioResult};

pub async fn run() -> anyhow::Result<ScenarioResult> {
    let mut result = ScenarioResult::new("partition");
    let start = Instant::now();

    // ── Spawn four nodes ─────────────────────────────────────────────
    let node_a = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;
    let node_b = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;
    let node_c = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;
    let node_d = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;

    let id_a = node_a.id();
    let id_b = node_b.id();
    let id_c = node_c.id();
    let id_d = node_d.id();

    let addr_a = node_a.addr();
    let addr_b = node_b.addr();
    let addr_c = node_c.addr();
    let addr_d = node_d.addr();

    eprintln!("Alice: {id_a}");
    eprintln!("Bob  : {id_b}");
    eprintln!("Carol: {id_c}");
    eprintln!("Dave : {id_d}");

    let channels_a = ProtocolRuntime::spawn(
        node_a,
        RuntimeConfig { username: "alice".into(), encryption: true, ..Default::default() },
    );
    let mut channels_b = ProtocolRuntime::spawn(
        node_b,
        RuntimeConfig { username: "bob".into(), encryption: true, ..Default::default() },
    );
    let mut channels_c = ProtocolRuntime::spawn(
        node_c,
        RuntimeConfig { username: "carol".into(), encryption: true, ..Default::default() },
    );
    let mut channels_d = ProtocolRuntime::spawn(
        node_d,
        RuntimeConfig { username: "dave".into(), encryption: true, ..Default::default() },
    );

    // ── Phase 1: Register intra-group peers only ─────────────────────
    let step = timed_step_async("register intra-group peers (partition active)", || async {
        // Group-A: Alice ↔ Bob
        channels_a.handle.add_peer_addr(addr_b.clone()).await;
        channels_b.handle.add_peer_addr(addr_a.clone()).await;
        // Group-B: Carol ↔ Dave
        channels_c.handle.add_peer_addr(addr_d.clone()).await;
        channels_d.handle.add_peer_addr(addr_c.clone()).await;
        tokio::time::sleep(Duration::from_millis(400)).await;
        Ok("partition established — no cross-group routes".into())
    })
    .await;
    result.add(step);

    // ── Intra-group-A: Alice → Bob ───────────────────────────────────
    let step = timed_step_async("intra-group-A: Alice → Bob", || async {
        channels_a
            .handle
            .send_message(id_b, b"hello-bob".to_vec())
            .await
            .map_err(|e| format!("send failed: {e}"))?;
        match recv_timeout(&mut channels_b.messages, Duration::from_secs(6)).await {
            Ok(_) => Ok("delivered within group-A".into()),
            Err(e) => Err(format!("timeout within group-A: {e}")),
        }
    })
    .await;
    result.add(step);

    // ── Intra-group-B: Carol → Dave ──────────────────────────────────
    let step = timed_step_async("intra-group-B: Carol → Dave", || async {
        channels_c
            .handle
            .send_message(id_d, b"hello-dave".to_vec())
            .await
            .map_err(|e| format!("send failed: {e}"))?;
        match recv_timeout(&mut channels_d.messages, Duration::from_secs(6)).await {
            Ok(_) => Ok("delivered within group-B".into()),
            Err(e) => Err(format!("timeout within group-B: {e}")),
        }
    })
    .await;
    result.add(step);

    // ── Cross-partition blocked: Alice → Carol ────────────────────────
    // No route exists — Alice never registered Carol's address.
    // The message should never arrive at Carol within the short window.
    let step = timed_step_async("cross-partition blocked: Alice → Carol", || async {
        let _ = channels_a
            .handle
            .send_message(id_c, b"cross-partition-probe".to_vec())
            .await;
        match recv_timeout(&mut channels_c.messages, Duration::from_secs(3)).await {
            Ok(_) => Err("unexpected delivery across partition boundary".into()),
            Err(_) => Ok("correctly blocked — no delivery across partition".into()),
        }
    })
    .await;
    result.add(step);

    // ── Phase 2: Heal partition ───────────────────────────────────────
    let step = timed_step_async("heal partition (add cross-group routes)", || async {
        channels_a.handle.add_peer_addr(addr_c.clone()).await;
        channels_a.handle.add_peer_addr(addr_d.clone()).await;
        channels_b.handle.add_peer_addr(addr_c.clone()).await;
        channels_b.handle.add_peer_addr(addr_d.clone()).await;
        channels_c.handle.add_peer_addr(addr_a.clone()).await;
        channels_c.handle.add_peer_addr(addr_b.clone()).await;
        channels_d.handle.add_peer_addr(addr_a.clone()).await;
        channels_d.handle.add_peer_addr(addr_b.clone()).await;
        tokio::time::sleep(Duration::from_millis(600)).await;
        Ok("cross-group routes registered — partition healed".into())
    })
    .await;
    result.add(step);

    // ── Post-heal: Alice → Carol ─────────────────────────────────────
    let step = timed_step_async("post-heal: Alice → Carol", || async {
        channels_a
            .handle
            .send_message(id_c, b"hello-carol-post-heal".to_vec())
            .await
            .map_err(|e| format!("send failed: {e}"))?;
        match recv_timeout(&mut channels_c.messages, Duration::from_secs(8)).await {
            Ok(_) => Ok("cross-partition delivery restored after heal".into()),
            Err(e) => Err(format!("timeout after heal (Alice→Carol): {e}")),
        }
    })
    .await;
    result.add(step);

    // ── Post-heal: Dave → Bob ─────────────────────────────────────────
    let step = timed_step_async("post-heal: Dave → Bob", || async {
        channels_d
            .handle
            .send_message(id_b, b"hello-bob-from-dave".to_vec())
            .await
            .map_err(|e| format!("send failed: {e}"))?;
        match recv_timeout(&mut channels_b.messages, Duration::from_secs(8)).await {
            Ok(_) => Ok("cross-partition delivery confirmed (Dave→Bob) after heal".into()),
            Err(e) => Err(format!("timeout after heal (Dave→Bob): {e}")),
        }
    })
    .await;
    result.add(step);

    // ── Shutdown ──────────────────────────────────────────────────────
    channels_a.handle.shutdown().await;
    channels_b.handle.shutdown().await;
    channels_c.handle.shutdown().await;
    channels_d.handle.shutdown().await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    result.finalize(start);
    Ok(result)
}
