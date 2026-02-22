/// Group lifecycle scenario — create group, invite member, join, send messages,
/// leave. Exercises the full GroupManager + GroupHub flow through ProtocolRuntime.
use std::time::{Duration, Instant};

use tom_protocol::{ProtocolEvent, ProtocolRuntime, RuntimeConfig};
use tom_transport::{TomNode, TomNodeConfig};

use crate::scenario_common::{recv_timeout, timed_step_async, ScenarioResult};

/// Number of group messages to exchange.
const MESSAGE_COUNT: u32 = 5;

pub async fn run() -> anyhow::Result<ScenarioResult> {
    let mut result = ScenarioResult::new("group-lifecycle");
    let start = Instant::now();

    // ── Spawn three nodes: Alice (creator), Bob (member), Hub ──────
    let node_a = TomNode::bind(TomNodeConfig::new()).await?;
    let node_b = TomNode::bind(TomNodeConfig::new()).await?;
    let node_hub = TomNode::bind(TomNodeConfig::new()).await?;

    let id_a = node_a.id();
    let id_b = node_b.id();
    let id_hub = node_hub.id();

    eprintln!("Alice : {id_a}");
    eprintln!("Bob   : {id_b}");
    eprintln!("Hub   : {id_hub}");

    let config_a = RuntimeConfig {
        username: "alice".into(),
        ..Default::default()
    };
    let config_b = RuntimeConfig {
        username: "bob".into(),
        ..Default::default()
    };
    let config_hub = RuntimeConfig {
        username: "hub-relay".into(),
        ..Default::default()
    };

    let mut channels_a = ProtocolRuntime::spawn(node_a, config_a);
    let mut channels_b = ProtocolRuntime::spawn(node_b, config_b);
    let channels_hub = ProtocolRuntime::spawn(node_hub, config_hub);

    // ── Register peers ─────────────────────────────────────────────
    let step = timed_step_async("register peers", || async {
        // Everyone needs to know the hub, and hub needs to know members
        channels_a.handle.add_peer(id_hub).await;
        channels_a.handle.add_peer(id_b).await;
        channels_b.handle.add_peer(id_hub).await;
        channels_b.handle.add_peer(id_a).await;
        channels_hub.handle.add_peer(id_a).await;
        channels_hub.handle.add_peer(id_b).await;
        tokio::time::sleep(Duration::from_secs(3)).await;
        Ok(String::new())
    })
    .await;
    result.add(step);

    // ── Alice creates a group with Bob invited ─────────────────────
    let step = timed_step_async("create group", || async {
        channels_a
            .handle
            .create_group("Stress Test Group".into(), id_hub, vec![id_b])
            .await
            .map_err(|e| format!("create_group failed: {e}"))?;

        // Wait for GroupCreated event on Alice's side
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            match recv_timeout(&mut channels_a.events, Duration::from_secs(1)).await {
                Ok(ProtocolEvent::GroupCreated { group }) => {
                    return Ok(format!("group {} created", group.group_id));
                }
                Ok(_) => continue,
                Err(_) => continue,
            }
        }
        Err("timeout waiting for GroupCreated event".into())
    })
    .await;

    // Extract group_id for later steps
    let group_id_str = step
        .detail
        .as_ref()
        .and_then(|d| d.strip_prefix("group "))
        .and_then(|d| d.strip_suffix(" created"))
        .unwrap_or("unknown")
        .to_string();
    result.add(step);

    // ── Bob receives invite and accepts ────────────────────────────
    let step = timed_step_async("bob accepts invite", || async {
        // Wait for invite event on Bob's side
        let deadline = Instant::now() + Duration::from_secs(10);
        let mut invite_group_id = None;
        while Instant::now() < deadline {
            match recv_timeout(&mut channels_b.events, Duration::from_secs(1)).await {
                Ok(ProtocolEvent::GroupInviteReceived { invite }) => {
                    invite_group_id = Some(invite.group_id.clone());
                    break;
                }
                Ok(_) => continue,
                Err(_) => continue,
            }
        }

        let gid = invite_group_id.ok_or("timeout waiting for invite")?;
        channels_b
            .handle
            .accept_invite(gid.clone())
            .await
            .map_err(|e| format!("accept_invite failed: {e}"))?;

        // Wait for GroupJoined event
        let deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < deadline {
            match recv_timeout(&mut channels_b.events, Duration::from_secs(1)).await {
                Ok(ProtocolEvent::GroupJoined { group_id, .. }) => {
                    return Ok(format!("bob joined {group_id}"));
                }
                Ok(_) => continue,
                Err(_) => continue,
            }
        }
        Err("timeout waiting for GroupJoined event".into())
    })
    .await;
    result.add(step);

    // ── Alice sends N group messages ───────────────────────────────
    let gid_for_send = group_id_str.clone();
    let step = timed_step_async("send group messages", || async {
        let gid = tom_protocol::GroupId::from(gid_for_send);
        for i in 0..MESSAGE_COUNT {
            channels_a
                .handle
                .send_group_message(gid.clone(), format!("group-msg-{i}"))
                .await
                .map_err(|e| format!("send_group_message failed: {e}"))?;
            // Small delay to avoid overwhelming
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        Ok(format!("{MESSAGE_COUNT} sent"))
    })
    .await;
    result.add(step);

    // ── Bob receives group messages ────────────────────────────────
    let step = timed_step_async("bob receives group messages", || async {
        let mut received = 0u32;
        let deadline = Instant::now() + Duration::from_secs(15);
        while received < MESSAGE_COUNT && Instant::now() < deadline {
            match recv_timeout(&mut channels_b.events, Duration::from_secs(2)).await {
                Ok(ProtocolEvent::GroupMessageReceived { .. }) => {
                    received += 1;
                }
                Ok(_) => continue,
                Err(_) => continue,
            }
        }
        if received == MESSAGE_COUNT {
            Ok(format!("{received}/{MESSAGE_COUNT} received"))
        } else {
            Err(format!("{received}/{MESSAGE_COUNT} received (expected all)"))
        }
    })
    .await;
    result.add(step);

    // ── Bob leaves the group ───────────────────────────────────────
    let gid_for_leave = group_id_str.clone();
    let step = timed_step_async("bob leaves group", || async {
        let gid = tom_protocol::GroupId::from(gid_for_leave);
        channels_b
            .handle
            .leave_group(gid)
            .await
            .map_err(|e| format!("leave_group failed: {e}"))?;
        // Give time for the leave to propagate
        tokio::time::sleep(Duration::from_secs(2)).await;
        Ok("left".into())
    })
    .await;
    result.add(step);

    // ── Shutdown ───────────────────────────────────────────────────
    channels_a.handle.shutdown().await;
    channels_b.handle.shutdown().await;
    channels_hub.handle.shutdown().await;
    tokio::time::sleep(Duration::from_millis(100)).await;

    result.finalize(start);
    Ok(result)
}
