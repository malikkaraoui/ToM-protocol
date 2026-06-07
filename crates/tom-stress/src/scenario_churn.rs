/// Churn scenario — node arrival and departure mid-stream.
///
/// Verifies that:
///  1. Pre-churn messaging works in a 3-node mesh.
///  2. Shutting down Bob does not break Alice ↔ Charlie messaging.
///  3. A restarted node (new identity, re-registered) can receive messages again.
use std::time::{Duration, Instant};

use tom_protocol::{ProtocolRuntime, RuntimeConfig};
use tom_transport::{TomNode, TomNodeConfig};

use crate::scenario_common::{recv_timeout, timed_step_async, ScenarioResult};

pub async fn run() -> anyhow::Result<ScenarioResult> {
    let mut result = ScenarioResult::new("churn");
    let start = Instant::now();

    // ── Spawn three nodes ────────────────────────────────────────────
    let node_a = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;
    let node_b = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;
    let node_c = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;

    let id_a = node_a.id();
    let id_b = node_b.id();
    let id_c = node_c.id();

    let addr_a = node_a.addr();
    let addr_b = node_b.addr();
    let addr_c = node_c.addr();

    eprintln!("Alice  : {id_a}");
    eprintln!("Bob    : {id_b}");
    eprintln!("Charlie: {id_c}");

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
        RuntimeConfig { username: "charlie".into(), encryption: true, ..Default::default() },
    );

    // ── Register full mesh ────────────────────────────────────────────
    let step = timed_step_async("register peers (full mesh)", || async {
        channels_a.handle.add_peer_addr(addr_b.clone()).await;
        channels_a.handle.add_peer_addr(addr_c.clone()).await;
        channels_b.handle.add_peer_addr(addr_a.clone()).await;
        channels_b.handle.add_peer_addr(addr_c.clone()).await;
        channels_c.handle.add_peer_addr(addr_a.clone()).await;
        channels_c.handle.add_peer_addr(addr_b.clone()).await;
        tokio::time::sleep(Duration::from_millis(400)).await;
        Ok("full mesh established".into())
    })
    .await;
    result.add(step);

    // ── Pre-churn: Alice → Bob ────────────────────────────────────────
    let step = timed_step_async("pre-churn: Alice → Bob (3 msgs)", || async {
        for i in 0..3u32 {
            channels_a
                .handle
                .send_message(id_b, format!("pre-churn-{i}").into_bytes())
                .await
                .map_err(|e| format!("send failed: {e}"))?;
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        let mut received = 0u32;
        let deadline = Instant::now() + Duration::from_secs(8);
        while received < 3 && Instant::now() < deadline {
            match recv_timeout(&mut channels_b.messages, Duration::from_secs(2)).await {
                Ok(_) => received += 1,
                Err(_) => continue,
            }
        }
        if received >= 2 {
            Ok(format!("{received}/3 received pre-churn"))
        } else {
            Err(format!("only {received}/3 received pre-churn"))
        }
    })
    .await;
    result.add(step);

    // ── Pre-churn: Alice → Charlie ────────────────────────────────────
    let step = timed_step_async("pre-churn: Alice → Charlie (3 msgs)", || async {
        for i in 0..3u32 {
            channels_a
                .handle
                .send_message(id_c, format!("pre-churn-c-{i}").into_bytes())
                .await
                .map_err(|e| format!("send failed: {e}"))?;
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        let mut received = 0u32;
        let deadline = Instant::now() + Duration::from_secs(8);
        while received < 3 && Instant::now() < deadline {
            match recv_timeout(&mut channels_c.messages, Duration::from_secs(2)).await {
                Ok(_) => received += 1,
                Err(_) => continue,
            }
        }
        if received >= 2 {
            Ok(format!("{received}/3 received pre-churn"))
        } else {
            Err(format!("only {received}/3 received pre-churn"))
        }
    })
    .await;
    result.add(step);

    // ── Churn: Bob leaves ─────────────────────────────────────────────
    let step = timed_step_async("churn: Bob node shutdown", || async {
        channels_b.handle.shutdown().await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        Ok("Bob left the network".into())
    })
    .await;
    result.add(step);

    // ── Resilience: Alice → Charlie unaffected by Bob's departure ─────
    let step = timed_step_async("resilience: Alice → Charlie (Bob down, 3 msgs)", || async {
        for i in 0..3u32 {
            channels_a
                .handle
                .send_message(id_c, format!("post-churn-{i}").into_bytes())
                .await
                .map_err(|e| format!("send failed: {e}"))?;
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        let mut received = 0u32;
        let deadline = Instant::now() + Duration::from_secs(8);
        while received < 3 && Instant::now() < deadline {
            match recv_timeout(&mut channels_c.messages, Duration::from_secs(2)).await {
                Ok(_) => received += 1,
                Err(_) => continue,
            }
        }
        if received >= 2 {
            Ok(format!("{received}/3 — Alice↔Charlie unaffected by Bob churn"))
        } else {
            Err(format!("only {received}/3 — Bob churn may have disrupted survivors"))
        }
    })
    .await;
    result.add(step);

    // ── Recovery: Bob restarts with a new identity ────────────────────
    let step = timed_step_async("recovery: Bob restarts (new node)", || async {
        let node_b2 = TomNode::bind(TomNodeConfig::new().n0_discovery(false))
            .await
            .map_err(|e| format!("bind failed: {e}"))?;
        let id_b2 = node_b2.id();
        let addr_b2 = node_b2.addr();
        eprintln!("Bob (restarted): {id_b2}");

        let mut channels_b2 = ProtocolRuntime::spawn(
            node_b2,
            RuntimeConfig { username: "bob-restarted".into(), encryption: true, ..Default::default() },
        );

        // Re-register with Alice
        channels_b2.handle.add_peer_addr(addr_a.clone()).await;
        channels_a.handle.add_peer_addr(addr_b2.clone()).await;
        tokio::time::sleep(Duration::from_millis(500)).await;

        channels_a
            .handle
            .send_message(id_b2, b"hello-bob-restarted".to_vec())
            .await
            .map_err(|e| format!("send failed: {e}"))?;

        let received = recv_timeout(&mut channels_b2.messages, Duration::from_secs(8)).await;

        channels_b2.handle.shutdown().await;
        tokio::time::sleep(Duration::from_millis(100)).await;

        match received {
            Ok(_) => Ok("Bob restart + delivery confirmed".into()),
            Err(e) => Err(format!("timeout after Bob restart: {e}")),
        }
    })
    .await;
    result.add(step);

    // ── Shutdown survivors ────────────────────────────────────────────
    channels_a.handle.shutdown().await;
    channels_c.handle.shutdown().await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    result.finalize(start);
    Ok(result)
}
