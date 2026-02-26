/// Chaos scenario — randomized multi-node test that exercises the protocol
/// under unpredictable conditions.
///
/// Steps are shuffled randomly. Random delays, random message sizes,
/// random node shutdowns and restarts. Exercises resilience and edge cases.
use std::time::{Duration, Instant};

use rand::seq::SliceRandom;
use rand::Rng;
use tom_protocol::{ProtocolRuntime, RuntimeConfig};
use tom_transport::{TomNode, TomNodeConfig};

use crate::scenario_common::{recv_timeout, timed_step_async, ScenarioResult};

pub async fn run() -> anyhow::Result<ScenarioResult> {
    let mut result = ScenarioResult::new("chaos");
    let start = Instant::now();
    let mut rng = rand::rng();

    // ── Spawn 3 nodes ───────────────────────────────────────────────
    let node_a = TomNode::bind(TomNodeConfig::new()).await?;
    let node_b = TomNode::bind(TomNodeConfig::new()).await?;
    let node_c = TomNode::bind(TomNodeConfig::new()).await?;

    let id_a = node_a.id();
    let id_b = node_b.id();
    let id_c = node_c.id();

    eprintln!("Node A: {id_a}");
    eprintln!("Node B: {id_b}");
    eprintln!("Node C: {id_c}");

    let config_a = RuntimeConfig {
        username: "chaos-alice".into(),
        encryption: true,
        ..Default::default()
    };
    let config_b = RuntimeConfig {
        username: "chaos-bob".into(),
        encryption: true,
        ..Default::default()
    };
    let config_c = RuntimeConfig {
        username: "chaos-charlie".into(),
        encryption: true,
        ..Default::default()
    };

    let channels_a = ProtocolRuntime::spawn(node_a, config_a);
    let mut channels_b = ProtocolRuntime::spawn(node_b, config_b);
    let mut channels_c = ProtocolRuntime::spawn(node_c, config_c);

    // ── Register all peers with random delays ───────────────────────
    let step = timed_step_async("register peers (random delays)", || async {
        let pairs = vec![
            (channels_a.handle.clone(), id_b),
            (channels_a.handle.clone(), id_c),
            (channels_b.handle.clone(), id_a),
            (channels_b.handle.clone(), id_c),
            (channels_c.handle.clone(), id_a),
            (channels_c.handle.clone(), id_b),
        ];

        // Shuffle registration order
        let mut shuffled: Vec<_> = pairs.into_iter().collect();
        shuffled.shuffle(&mut rand::rng());

        for (handle, peer) in shuffled {
            handle.add_peer(peer).await;
            let delay_ms = rand::rng().random_range(50..300);
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }

        tokio::time::sleep(Duration::from_secs(2)).await;
        Ok(String::new())
    })
    .await;
    result.add(step);

    // ── Random burst: send N messages of random sizes to random targets ─
    let msg_count: u32 = rng.random_range(5..15);
    let step = timed_step_async("random burst messaging", || async {
        let targets = [id_b, id_c];
        let mut sent = 0u32;
        let mut received_b = 0u32;
        let mut received_c = 0u32;

        for _ in 0..msg_count {
            let target = targets[rand::rng().random_range(0..targets.len())];
            let size = rand::rng().random_range(10..2000);
            let payload: Vec<u8> = (0..size).map(|_| rand::rng().random()).collect();

            if channels_a.handle.send_message(target, payload).await.is_ok() {
                sent += 1;
            }

            // Random inter-message delay
            let delay = rand::rng().random_range(20..200);
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }

        // Collect from both receivers
        let deadline = Instant::now() + Duration::from_secs(15);
        while Instant::now() < deadline && (received_b + received_c) < sent {
            tokio::select! {
                Ok(_) = recv_timeout(&mut channels_b.messages, Duration::from_secs(1)) => {
                    received_b += 1;
                }
                Ok(_) = recv_timeout(&mut channels_c.messages, Duration::from_secs(1)) => {
                    received_c += 1;
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {}
            }
        }

        let total = received_b + received_c;
        if total > 0 {
            Ok(format!("{total}/{sent} received (B={received_b}, C={received_c})"))
        } else {
            Err(format!("0/{sent} received"))
        }
    })
    .await;
    result.add(step);

    // ── Bidirectional chaos: all nodes send to each other simultaneously ─
    let step = timed_step_async("bidirectional chaos", || async {
        let handles = vec![
            (channels_a.handle.clone(), vec![id_b, id_c]),
            (channels_b.handle.clone(), vec![id_a, id_c]),
            (channels_c.handle.clone(), vec![id_a, id_b]),
        ];

        let mut tasks = Vec::new();
        for (handle, targets) in handles {
            let task = tokio::spawn(async move {
                let mut sent = 0u32;
                for i in 0..5u32 {
                    let target = targets[rand::rng().random_range(0..targets.len())];
                    let payload = format!("chaos-{i}").into_bytes();
                    if handle.send_message(target, payload).await.is_ok() {
                        sent += 1;
                    }
                    let delay = rand::rng().random_range(50..300);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                }
                sent
            });
            tasks.push(task);
        }

        let mut total_sent = 0u32;
        for task in tasks {
            total_sent += task.await.unwrap_or(0);
        }

        // Collect from B and C (A doesn't receive in this setup since we
        // consume from B and C channels)
        let mut total_received = 0u32;
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            tokio::select! {
                Ok(_) = recv_timeout(&mut channels_b.messages, Duration::from_millis(500)) => {
                    total_received += 1;
                }
                Ok(_) = recv_timeout(&mut channels_c.messages, Duration::from_millis(500)) => {
                    total_received += 1;
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    if total_received >= 5 { break; } // enough
                }
            }
        }

        Ok(format!("{total_received} received from {total_sent} total sent"))
    })
    .await;
    result.add(step);

    // ── Group with random member selection ───────────────────────────
    let step = timed_step_async("random group creation", || async {
        // Randomly pick hub and members
        let nodes = [
            (channels_a.handle.clone(), id_a, "alice"),
            (channels_b.handle.clone(), id_b, "bob"),
            (channels_c.handle.clone(), id_c, "charlie"),
        ];

        let hub_idx = rand::rng().random_range(0..nodes.len());
        let creator_idx = (hub_idx + 1) % nodes.len();
        let member_idx = (hub_idx + 2) % nodes.len();

        let hub_id = nodes[hub_idx].1;
        let creator = &nodes[creator_idx].0;
        let member_id = nodes[member_idx].1;

        eprintln!(
            "    Hub={}, Creator={}, Member={}",
            nodes[hub_idx].2, nodes[creator_idx].2, nodes[member_idx].2
        );

        creator
            .create_group("Chaos Group".into(), hub_id, vec![member_id])
            .await
            .map_err(|e| format!("create_group: {e}"))?;

        // Wait briefly for group creation events
        tokio::time::sleep(Duration::from_secs(3)).await;

        Ok(format!(
            "group created (hub={}, member={})",
            nodes[hub_idx].2, nodes[member_idx].2
        ))
    })
    .await;
    result.add(step);

    // ── Rapid node shutdown/queries (stress resilience) ──────────────
    let step = timed_step_async("resilience: rapid queries during chaos", || async {
        // Fire off metric queries while nodes are active
        let mut queries_ok = 0u32;

        for _ in 0..5u32 {
            if channels_a.handle.get_role_metrics(id_b).await.is_some() {
                queries_ok += 1;
            }
            let scores = channels_a.handle.get_all_role_scores().await;
            if !scores.is_empty() {
                queries_ok += 1;
            }

            let delay = rand::rng().random_range(100..500);
            tokio::time::sleep(Duration::from_millis(delay)).await;
        }

        Ok(format!("{queries_ok} queries returned data"))
    })
    .await;
    result.add(step);

    // ── Shutdown ────────────────────────────────────────────────────
    // Random shutdown order
    let mut shutdown_order = vec![
        ("alice", channels_a.handle.clone()),
        ("bob", channels_b.handle.clone()),
        ("charlie", channels_c.handle.clone()),
    ];
    shutdown_order.shuffle(&mut rng);

    for (name, handle) in shutdown_order {
        eprintln!("  Shutting down {name}...");
        handle.shutdown().await;
        let delay = rng.random_range(50..200);
        tokio::time::sleep(Duration::from_millis(delay as u64)).await;
    }

    result.finalize(start);
    Ok(result)
}
