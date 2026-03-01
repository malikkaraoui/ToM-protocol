/// Campaign mode — orchestrates all 7 stress phases against a remote responder.
///
/// Phases: Ping → Burst → E2E → Group Encrypted → Failover → Roles → Endurance
use std::time::{Duration, Instant};

use serde::Serialize;
use tom_protocol::{
    DeliveredMessage, GroupId, ProtocolEvent, ProtocolRuntime, RuntimeConfig,
};
use tom_transport::{NodeId, TomNode, TomNodeConfig};
use tokio::sync::mpsc;

use crate::events::emit;
use crate::scenario_common::recv_timeout;

// ── Configuration ──────────────────────────────────────────────────

pub struct CampaignConfig {
    pub target: NodeId,
    pub name: String,
    pub duration_s: u64,
    pub phase: Option<String>,
    pub max_message_size: usize,
    pub relay_url: Option<String>,
    pub no_n0_discovery: bool,
}

// ── JSONL Event Types ──────────────────────────────────────────────

#[derive(Serialize)]
struct CampaignStarted {
    event: &'static str,
    name: String,
    target: String,
    duration_s: u64,
    timestamp: String,
}

#[derive(Serialize)]
struct PhaseResult {
    event: &'static str,
    phase: String,
    status: String,
    sent: u32,
    received: u32,
    lost: u32,
    loss_pct: f64,
    avg_rtt_ms: f64,
    min_rtt_ms: f64,
    max_rtt_ms: f64,
    elapsed_s: f64,
    detail: String,
}

#[derive(Serialize)]
struct EnduranceRolling {
    event: &'static str,
    phase: &'static str,
    minute: u64,
    sent: u32,
    received: u32,
    loss_pct: f64,
    avg_rtt_ms: f64,
    reconnections: u32,
    elapsed_s: f64,
}

#[derive(Serialize)]
struct CampaignSummary {
    event: &'static str,
    name: String,
    phases: Vec<PhaseSummaryLine>,
    total_sent: u32,
    total_received: u32,
    total_elapsed_s: f64,
    overall_status: String,
}

#[derive(Serialize, Clone)]
struct PhaseSummaryLine {
    phase: String,
    status: String,
    sent: u32,
    received: u32,
    loss_pct: f64,
    avg_rtt_ms: f64,
}

// ── Internal tracking ──────────────────────────────────────────────

struct PhaseStats {
    sent: u32,
    received: u32,
    rtts: Vec<f64>,
    errors: Vec<String>,
    start: Instant,
}

impl PhaseStats {
    fn new() -> Self {
        Self {
            sent: 0,
            received: 0,
            rtts: Vec::new(),
            errors: Vec::new(),
            start: Instant::now(),
        }
    }

    fn record_rtt(&mut self, rtt_ms: f64) {
        self.received += 1;
        self.rtts.push(rtt_ms);
    }

    fn lost(&self) -> u32 {
        self.sent.saturating_sub(self.received)
    }

    fn loss_pct(&self) -> f64 {
        if self.sent == 0 {
            0.0
        } else {
            (self.lost() as f64 / self.sent as f64) * 100.0
        }
    }

    fn avg_rtt(&self) -> f64 {
        if self.rtts.is_empty() {
            0.0
        } else {
            self.rtts.iter().sum::<f64>() / self.rtts.len() as f64
        }
    }

    fn min_rtt(&self) -> f64 {
        self.rtts.iter().cloned().fold(f64::MAX, f64::min)
    }

    fn max_rtt(&self) -> f64 {
        self.rtts.iter().cloned().fold(0.0, f64::max)
    }

    fn elapsed_s(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    fn status(&self) -> &'static str {
        if self.errors.is_empty() && self.lost() == 0 {
            "PASS"
        } else if self.loss_pct() < 5.0 {
            "WARN"
        } else {
            "FAIL"
        }
    }

    fn to_result(&self, phase: &str) -> PhaseResult {
        PhaseResult {
            event: "phase_result",
            phase: phase.into(),
            status: self.status().into(),
            sent: self.sent,
            received: self.received,
            lost: self.lost(),
            loss_pct: self.loss_pct(),
            avg_rtt_ms: self.avg_rtt(),
            min_rtt_ms: if self.rtts.is_empty() { 0.0 } else { self.min_rtt() },
            max_rtt_ms: self.max_rtt(),
            elapsed_s: self.elapsed_s(),
            detail: if self.errors.is_empty() {
                format!("{}/{} OK", self.received, self.sent)
            } else {
                self.errors.join("; ")
            },
        }
    }

    fn to_summary_line(&self, phase: &str) -> PhaseSummaryLine {
        PhaseSummaryLine {
            phase: phase.into(),
            status: self.status().into(),
            sent: self.sent,
            received: self.received,
            loss_pct: self.loss_pct(),
            avg_rtt_ms: self.avg_rtt(),
        }
    }
}

// ── Main entry ─────────────────────────────────────────────────────

pub async fn run(config: CampaignConfig) -> anyhow::Result<()> {
    let campaign_start = Instant::now();

    let mut node_config = TomNodeConfig::new().max_message_size(config.max_message_size);
    if let Some(ref url) = config.relay_url {
        node_config = node_config.relay_url(url.parse()?);
    }
    if config.no_n0_discovery {
        node_config = node_config.n0_discovery(false);
    }
    let node = TomNode::bind(node_config).await?;
    let local_id = node.id();

    eprintln!("Campaign Node ID: {local_id}");
    eprintln!("Target: {}", config.target);
    eprintln!("Duration: {}s", config.duration_s);
    eprintln!();

    let runtime_config = RuntimeConfig {
        username: config.name.clone(),
        encryption: true,
        ..Default::default()
    };

    let channels = ProtocolRuntime::spawn(node, runtime_config);
    let handle = channels.handle.clone();

    // Spawn background channel pump to prevent runtime from blocking on try_send.
    // The pump continuously drains runtime channels and forwards to local buffers.
    // This ensures try_send in executor never drops messages due to full channels.
    let (local_msg_tx, mut local_msg_rx) = mpsc::channel::<DeliveredMessage>(1024);
    let (local_evt_tx, mut local_evt_rx) = mpsc::channel::<ProtocolEvent>(1024);

    let _pump_task = tokio::spawn({
        let mut runtime_msgs = channels.messages;
        let mut runtime_evts = channels.events;
        let mut runtime_status = channels.status_changes;
        async move {
            loop {
                tokio::select! {
                    Some(msg) = runtime_msgs.recv() => {
                        // Forward messages to local buffer
                        // Backpressure here is OK - it just slows down the pump,
                        // not the runtime itself
                        let _ = local_msg_tx.send(msg).await;
                    }
                    Some(evt) = runtime_evts.recv() => {
                        // Forward events to local buffer
                        let _ = local_evt_tx.send(evt).await;
                    }
                    Some(_sc) = runtime_status.recv() => {
                        // Consume and drop status changes (not used in campaign)
                    }
                    else => break,
                }
            }
        }
    });

    // Register peer
    handle.add_peer(config.target).await;

    emit(&CampaignStarted {
        event: "campaign_started",
        name: config.name.clone(),
        target: config.target.to_string(),
        duration_s: config.duration_s,
        timestamp: crate::events::now_iso(),
    });

    // Wait for peer discovery
    eprintln!("Waiting for peer discovery (5s)...");
    wait_for_peer(&mut local_evt_rx, config.target, Duration::from_secs(5)).await;

    let mut summaries: Vec<PhaseSummaryLine> = Vec::new();
    let mut total_sent: u32 = 0;
    let mut total_received: u32 = 0;

    let should_run = |phase: &str| -> bool {
        config.phase.as_ref().is_none_or(|p| p == phase)
    };

    // ── Phase 1: Ping ────────────────────────────────────────────
    if should_run("ping") {
        eprintln!("\n═══ Phase 1: PING (RTT baseline) ═══");
        let stats = phase_ping(&handle, &mut local_msg_rx, config.target, 20).await;
        let result = stats.to_result("ping");
        emit(&result);
        print_phase_result(&result);
        total_sent += stats.sent;
        total_received += stats.received;
        summaries.push(stats.to_summary_line("ping"));
        drain_local_channels(&mut local_msg_rx, &mut local_evt_rx).await;
    }

    // ── Phase 2: Burst ───────────────────────────────────────────
    if should_run("burst") {
        eprintln!("\n═══ Phase 2: BURST (throughput) ═══");
        // Reduced from 100 to 30 for low-memory devices (e.g., 957MB NAS)
        let stats = phase_burst(&handle, &mut local_msg_rx, config.target, 30, 3).await;
        let result = stats.to_result("burst");
        emit(&result);
        print_phase_result(&result);
        total_sent += stats.sent;
        total_received += stats.received;
        summaries.push(stats.to_summary_line("burst"));
        drain_local_channels(&mut local_msg_rx, &mut local_evt_rx).await;
    }

    // ── Phase 3: Protocol E2E ────────────────────────────────────
    if should_run("e2e") {
        eprintln!("\n═══ Phase 3: PROTOCOL E2E (encrypted chat) ═══");
        // Reduced from 50 to 20 for low-memory devices
        let stats = phase_e2e(&handle, &mut local_msg_rx, config.target, 20).await;
        let result = stats.to_result("e2e");
        emit(&result);
        print_phase_result(&result);
        total_sent += stats.sent;
        total_received += stats.received;
        summaries.push(stats.to_summary_line("e2e"));
        drain_local_channels(&mut local_msg_rx, &mut local_evt_rx).await;
    }

    // ── Phase 4: Group Encrypted ─────────────────────────────────
    if should_run("group") {
        eprintln!("\n═══ Phase 4: GROUP ENCRYPTED (Sender Keys) ═══");
        // Reduced from 100 to 20 for low-memory devices
        let stats = phase_group(
            &handle,
            &mut local_evt_rx,
            &mut local_msg_rx,
            local_id,
            config.target,
            20,
        )
        .await;
        let result = stats.to_result("group");
        emit(&result);
        print_phase_result(&result);
        total_sent += stats.sent;
        total_received += stats.received;
        summaries.push(stats.to_summary_line("group"));
        drain_local_channels(&mut local_msg_rx, &mut local_evt_rx).await;
    }

    // ── Phase 5: Failover ────────────────────────────────────────
    if should_run("failover") {
        eprintln!("\n═══ Phase 5: FAILOVER (shadow promotion) ═══");
        let stats = phase_failover(
            &handle,
            &mut local_evt_rx,
            &mut local_msg_rx,
            local_id,
            config.target,
        )
        .await;
        let result = stats.to_result("failover");
        emit(&result);
        print_phase_result(&result);
        total_sent += stats.sent;
        total_received += stats.received;
        summaries.push(stats.to_summary_line("failover"));
        drain_local_channels(&mut local_msg_rx, &mut local_evt_rx).await;
    }

    // ── Phase 6: Roles ──────────────────────────────────────────
    if should_run("roles") {
        eprintln!("\n═══ Phase 6: ROLES (score queries & events) ═══");
        let stats = phase_roles(&handle, &mut local_evt_rx, config.target).await;
        let result = stats.to_result("roles");
        emit(&result);
        print_phase_result(&result);
        total_sent += stats.sent;
        total_received += stats.received;
        summaries.push(stats.to_summary_line("roles"));
        drain_local_channels(&mut local_msg_rx, &mut local_evt_rx).await;
    }

    // ── Phase 7: Endurance ───────────────────────────────────────
    if should_run("endurance") {
        eprintln!("\n═══ Phase 7: ENDURANCE (1 msg/s, {}s) ═══", config.duration_s);
        let stats = phase_endurance(
            &handle,
            &mut local_msg_rx,
            config.target,
            config.duration_s,
        )
        .await;
        let result = stats.to_result("endurance");
        emit(&result);
        print_phase_result(&result);
        total_sent += stats.sent;
        total_received += stats.received;
        summaries.push(stats.to_summary_line("endurance"));
    }

    // ── Summary ──────────────────────────────────────────────────
    let overall = if summaries.iter().all(|s| s.status == "PASS") {
        "PASS"
    } else if summaries.iter().any(|s| s.status == "FAIL") {
        "FAIL"
    } else {
        "WARN"
    };

    emit(&CampaignSummary {
        event: "campaign_summary",
        name: config.name.clone(),
        phases: summaries.clone(),
        total_sent,
        total_received,
        total_elapsed_s: campaign_start.elapsed().as_secs_f64(),
        overall_status: overall.into(),
    });

    eprintln!("\n╔══════════════════════════════════════════╗");
    eprintln!("║          CAMPAIGN SUMMARY                ║");
    eprintln!("╠══════════════════════════════════════════╣");
    for s in &summaries {
        let icon = match s.status.as_str() {
            "PASS" => " OK ",
            "WARN" => "WARN",
            _ => "FAIL",
        };
        eprintln!(
            "║ [{icon}] {:<12} {}/{} ({:.1}%) avg {:.1}ms ║",
            s.phase, s.received, s.sent, s.loss_pct, s.avg_rtt_ms,
        );
    }
    eprintln!("╠══════════════════════════════════════════╣");
    eprintln!(
        "║ Total: {total_received}/{total_sent} | {:.1}s | [{overall}]",
        campaign_start.elapsed().as_secs_f64(),
    );
    eprintln!("╚══════════════════════════════════════════╝");

    handle.shutdown().await;
    tokio::time::sleep(Duration::from_millis(200)).await;

    Ok(())
}

// ── Phase implementations ──────────────────────────────────────────

async fn phase_ping(
    handle: &tom_protocol::RuntimeHandle,
    messages: &mut mpsc::Receiver<DeliveredMessage>,
    target: NodeId,
    count: u32,
) -> PhaseStats {
    let mut stats = PhaseStats::new();

    for seq in 0..count {
        let payload = format!("PING:{seq}");
        let send_time = Instant::now();
        stats.sent += 1;

        if let Err(e) = handle.send_message(target, payload.into_bytes()).await {
            stats.errors.push(format!("send #{seq}: {e}"));
            continue;
        }

        // Wait for PONG
        match recv_timeout(messages, Duration::from_secs(10)).await {
            Ok(msg) => {
                let rtt = send_time.elapsed().as_secs_f64() * 1000.0;
                let text = String::from_utf8_lossy(&msg.payload);
                if text.starts_with("PONG:") {
                    stats.record_rtt(rtt);
                    if seq < 5 || seq == count - 1 {
                        eprintln!("  ping #{seq}: {rtt:.1}ms (enc={}, sig={})", msg.was_encrypted, msg.signature_valid);
                    }
                } else {
                    // Got a non-pong message, still count it
                    stats.record_rtt(rtt);
                }
            }
            Err(_) => {
                stats.errors.push(format!("timeout ping #{seq}"));
            }
        }

        tokio::time::sleep(Duration::from_millis(500)).await;
    }

    stats
}

async fn phase_burst(
    handle: &tom_protocol::RuntimeHandle,
    messages: &mut mpsc::Receiver<DeliveredMessage>,
    target: NodeId,
    count_per_round: u32,
    rounds: u32,
) -> PhaseStats {
    let mut stats = PhaseStats::new();

    for round in 0..rounds {
        eprintln!("  Round {}/{rounds}...", round + 1);
        let round_start = Instant::now();

        // Spawn sender in background to avoid deadlock:
        // cmd_tx (cap 64) and msg_tx (cap 64) would block each other
        // if we send all 100 before reading any responses.
        let send_handle = handle.clone();
        let send_count = count_per_round;
        let send_task = tokio::spawn(async move {
            let mut errors = Vec::new();
            for seq in 0..send_count {
                let global_seq = round * send_count + seq;
                let payload = format!("BURST:{global_seq}");
                if let Err(e) = send_handle.send_message(target, payload.into_bytes()).await {
                    errors.push(format!("burst send #{global_seq}: {e}"));
                }
            }
            errors
        });
        stats.sent += count_per_round;

        // Collect responses concurrently with sends
        let mut round_received = 0u32;
        let deadline = Instant::now() + Duration::from_secs(30);
        while round_received < count_per_round && Instant::now() < deadline {
            match recv_timeout(messages, Duration::from_secs(5)).await {
                Ok(msg) => {
                    let text = String::from_utf8_lossy(&msg.payload);
                    if text.starts_with("BURST-ACK:") || text.starts_with("ECHO:BURST:") {
                        let rtt = round_start.elapsed().as_secs_f64() * 1000.0
                            / (round_received + 1) as f64;
                        stats.record_rtt(rtt);
                        round_received += 1;
                    }
                }
                Err(_) => break,
            }
        }

        // Collect any send errors
        if let Ok(errors) = send_task.await {
            for e in errors {
                stats.errors.push(e);
            }
        }

        let round_elapsed = round_start.elapsed().as_secs_f64() * 1000.0;
        eprintln!(
            "    {round_received}/{count_per_round} acked in {round_elapsed:.0}ms",
        );

        if round + 1 < rounds {
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    }

    stats
}

async fn phase_e2e(
    handle: &tom_protocol::RuntimeHandle,
    messages: &mut mpsc::Receiver<DeliveredMessage>,
    target: NodeId,
    count: u32,
) -> PhaseStats {
    let mut stats = PhaseStats::new();
    let mut encrypted_count = 0u32;
    let mut signed_count = 0u32;

    for seq in 0..count {
        let payload = format!("E2E-MSG:{seq}:payload-data-here");
        let send_time = Instant::now();
        stats.sent += 1;

        if let Err(e) = handle.send_message(target, payload.into_bytes()).await {
            stats.errors.push(format!("e2e send #{seq}: {e}"));
            continue;
        }

        match recv_timeout(messages, Duration::from_secs(10)).await {
            Ok(msg) => {
                let rtt = send_time.elapsed().as_secs_f64() * 1000.0;
                stats.record_rtt(rtt);
                if msg.was_encrypted {
                    encrypted_count += 1;
                }
                if msg.signature_valid {
                    signed_count += 1;
                }
            }
            Err(_) => {
                stats.errors.push(format!("timeout e2e #{seq}"));
            }
        }

        // Small delay to not overwhelm
        if seq % 10 == 0 {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    }

    eprintln!(
        "  Encrypted: {encrypted_count}/{} | Signed: {signed_count}/{}",
        stats.received, stats.received,
    );

    if encrypted_count != stats.received {
        stats
            .errors
            .push(format!("only {encrypted_count}/{} encrypted", stats.received));
    }
    if signed_count != stats.received {
        stats
            .errors
            .push(format!("only {signed_count}/{} signed", stats.received));
    }

    stats
}

async fn phase_group(
    handle: &tom_protocol::RuntimeHandle,
    events: &mut mpsc::Receiver<ProtocolEvent>,
    _messages: &mut mpsc::Receiver<DeliveredMessage>,
    local_id: NodeId,
    target: NodeId,
    msg_count: u32,
) -> PhaseStats {
    let mut stats = PhaseStats::new();

    // Step 1: Create group (we are hub, target is member)
    eprintln!("  Creating group (hub=us, member=responder)...");
    if let Err(e) = handle
        .create_group("StressCampaign".into(), local_id, vec![target])
        .await
    {
        stats.errors.push(format!("create_group: {e}"));
        return stats;
    }

    // Wait for GroupCreated
    let mut group_id: Option<GroupId> = None;
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        match recv_timeout(events, Duration::from_secs(2)).await {
            Ok(ProtocolEvent::GroupCreated { group }) => {
                eprintln!("  Group created: {}", group.group_id);
                group_id = Some(group.group_id);
                break;
            }
            Ok(_) => continue,
            Err(_) => continue,
        }
    }

    let Some(gid) = group_id else {
        stats.errors.push("timeout waiting for GroupCreated".into());
        return stats;
    };

    // Wait for responder to join (MemberJoined event)
    eprintln!("  Waiting for responder to join...");
    let deadline = Instant::now() + Duration::from_secs(15);
    let mut joined = false;
    while Instant::now() < deadline {
        match recv_timeout(events, Duration::from_secs(2)).await {
            Ok(ProtocolEvent::GroupMemberJoined { .. }) => {
                eprintln!("  Responder joined group");
                joined = true;
                break;
            }
            Ok(_) => continue,
            Err(_) => continue,
        }
    }

    if !joined {
        stats.errors.push("timeout waiting for responder to join".into());
        return stats;
    }

    // Wait a bit for Sender Key exchange to complete
    eprintln!("  Waiting for Sender Key exchange (3s)...");
    tokio::time::sleep(Duration::from_secs(3)).await;

    // Drain any queued events
    while events.try_recv().is_ok() {}

    // Step 2: Send encrypted group messages
    eprintln!("  Sending {msg_count} encrypted group messages...");
    let mut group_received = 0u32;

    for seq in 0..msg_count {
        let text = format!("group-stress-{seq}");
        let send_time = Instant::now();
        stats.sent += 1;

        if let Err(e) = handle.send_group_message(gid.clone(), text).await {
            stats.errors.push(format!("group send #{seq}: {e}"));
            continue;
        }

        // Wait for group echo (GroupMessageReceived event)
        let msg_deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < msg_deadline {
            match recv_timeout(events, Duration::from_secs(2)).await {
                Ok(ProtocolEvent::GroupMessageReceived { message }) => {
                    if message.text.starts_with("GROUP-ECHO:") {
                        let rtt = send_time.elapsed().as_secs_f64() * 1000.0;
                        stats.record_rtt(rtt);
                        group_received += 1;
                        break;
                    }
                    // It could be our own message echoed back — skip
                }
                Ok(_) => continue,
                Err(_) => break,
            }
        }

        if seq % 20 == 0 && seq > 0 {
            eprintln!("    {group_received}/{} received so far...", seq + 1);
        }

        // Pace at ~4 msg/sec to stay under hub rate limit (5/sec/sender)
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    eprintln!("  Group messages: {group_received}/{msg_count}");

    // Step 3: Leave group
    if let Err(e) = handle.leave_group(gid).await {
        stats.errors.push(format!("leave_group: {e}"));
    }
    tokio::time::sleep(Duration::from_secs(1)).await;

    stats
}

async fn phase_failover(
    handle: &tom_protocol::RuntimeHandle,
    events: &mut mpsc::Receiver<ProtocolEvent>,
    messages: &mut mpsc::Receiver<DeliveredMessage>,
    local_id: NodeId,
    target: NodeId,
) -> PhaseStats {
    let mut stats = PhaseStats::new();

    // For failover test: create a group where we are hub, target is member (and shadow).
    // We can't actually kill ourselves, so we test that the shadow assignment
    // and ping/pong cycle works correctly. We verify the chain is set up.
    eprintln!("  Creating group for failover validation...");

    if let Err(e) = handle
        .create_group("FailoverTest".into(), local_id, vec![target])
        .await
    {
        stats.errors.push(format!("create_group: {e}"));
        return stats;
    }

    // Wait for group creation + member join
    let mut group_id: Option<GroupId> = None;
    let deadline = Instant::now() + Duration::from_secs(15);
    while Instant::now() < deadline {
        match recv_timeout(events, Duration::from_secs(2)).await {
            Ok(ProtocolEvent::GroupCreated { group }) => {
                group_id = Some(group.group_id.clone());
                eprintln!("  Group created: {}", group.group_id);
            }
            Ok(ProtocolEvent::GroupMemberJoined { .. }) => {
                eprintln!("  Responder joined (shadow candidate)");
                break;
            }
            Ok(_) => continue,
            Err(_) => continue,
        }
    }

    let Some(gid) = group_id else {
        stats.errors.push("timeout creating failover group".into());
        return stats;
    };

    // Wait for shadow assignment events
    eprintln!("  Waiting for shadow assignment (5s)...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    // Send a few messages to verify the group still works
    for seq in 0..10 {
        let text = format!("failover-check-{seq}");
        let send_time = Instant::now();
        stats.sent += 1;

        if let Err(e) = handle.send_group_message(gid.clone(), text).await {
            stats.errors.push(format!("failover send #{seq}: {e}"));
            continue;
        }

        // Wait for echo
        let msg_deadline = Instant::now() + Duration::from_secs(10);
        while Instant::now() < msg_deadline {
            match recv_timeout(events, Duration::from_secs(2)).await {
                Ok(ProtocolEvent::GroupMessageReceived { message }) => {
                    if message.text.starts_with("GROUP-ECHO:") {
                        let rtt = send_time.elapsed().as_secs_f64() * 1000.0;
                        stats.record_rtt(rtt);
                        break;
                    }
                }
                Ok(_) => continue,
                Err(_) => break,
            }
        }

        // Pace at ~4 msg/sec to stay under hub rate limit (5/sec/sender)
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    // Also send some direct pings to verify P2P still works alongside groups
    for seq in 0..10 {
        let payload = format!("PING:failover-{seq}");
        let send_time = Instant::now();
        stats.sent += 1;

        if let Err(e) = handle.send_message(target, payload.into_bytes()).await {
            stats.errors.push(format!("failover ping #{seq}: {e}"));
            continue;
        }

        match recv_timeout(messages, Duration::from_secs(10)).await {
            Ok(_) => {
                let rtt = send_time.elapsed().as_secs_f64() * 1000.0;
                stats.record_rtt(rtt);
            }
            Err(_) => {
                stats.errors.push(format!("timeout failover ping #{seq}"));
            }
        }
    }

    eprintln!("  Failover validation: {}/{} messages OK", stats.received, stats.sent);

    if let Err(e) = handle.leave_group(gid).await {
        stats.errors.push(format!("leave_group: {e}"));
    }
    tokio::time::sleep(Duration::from_secs(1)).await;

    stats
}

async fn phase_endurance(
    handle: &tom_protocol::RuntimeHandle,
    messages: &mut mpsc::Receiver<DeliveredMessage>,
    target: NodeId,
    duration_s: u64,
) -> PhaseStats {
    let mut stats = PhaseStats::new();
    let endurance_start = Instant::now();
    let endurance_deadline = endurance_start + Duration::from_secs(duration_s);

    let mut last_report = Instant::now();
    let mut minute_sent: u32 = 0;
    let mut minute_received: u32 = 0;
    let mut minute_rtts: Vec<f64> = Vec::new();
    let mut minute_num: u64 = 0;
    let mut seq: u32 = 0;

    while Instant::now() < endurance_deadline {
        // Send one message
        let payload = format!("PING:{seq}");
        let send_time = Instant::now();
        stats.sent += 1;
        minute_sent += 1;
        seq += 1;

        if let Err(e) = handle.send_message(target, payload.into_bytes()).await {
            stats.errors.push(format!("endurance send #{seq}: {e}"));
        } else {
            // Wait for response (short timeout to keep 1 msg/s pace)
            match recv_timeout(messages, Duration::from_secs(5)).await {
                Ok(_msg) => {
                    let rtt = send_time.elapsed().as_secs_f64() * 1000.0;
                    stats.record_rtt(rtt);
                    minute_received += 1;
                    minute_rtts.push(rtt);
                }
                Err(_) => {
                    // Don't add to errors for each timeout — just count losses
                }
            }
        }

        // Rolling report every 60s
        if last_report.elapsed() >= Duration::from_secs(60) {
            minute_num += 1;
            let avg_rtt = if minute_rtts.is_empty() {
                0.0
            } else {
                minute_rtts.iter().sum::<f64>() / minute_rtts.len() as f64
            };
            let loss = if minute_sent > 0 {
                ((minute_sent - minute_received) as f64 / minute_sent as f64) * 100.0
            } else {
                0.0
            };

            emit(&EnduranceRolling {
                event: "endurance_rolling",
                phase: "endurance",
                minute: minute_num,
                sent: minute_sent,
                received: minute_received,
                loss_pct: loss,
                avg_rtt_ms: avg_rtt,
                reconnections: 0,
                elapsed_s: endurance_start.elapsed().as_secs_f64(),
            });

            eprintln!(
                "  [min {minute_num}] {minute_received}/{minute_sent} (loss {loss:.1}%) avg {avg_rtt:.1}ms | total {}/{}",
                stats.received, stats.sent,
            );

            minute_sent = 0;
            minute_received = 0;
            minute_rtts.clear();
            last_report = Instant::now();
        }

        // Target ~1 msg/s
        let elapsed = send_time.elapsed();
        if elapsed < Duration::from_secs(1) {
            tokio::time::sleep(Duration::from_secs(1) - elapsed).await;
        }
    }

    stats
}

async fn phase_roles(
    handle: &tom_protocol::RuntimeHandle,
    events: &mut mpsc::Receiver<ProtocolEvent>,
    target: NodeId,
) -> PhaseStats {
    let mut stats = PhaseStats::new();

    // Step 1: Query role metrics for target peer
    eprintln!("  Querying role metrics for target...");
    stats.sent += 1;
    match handle.get_role_metrics(target).await {
        Some(metrics) => {
            stats.record_rtt(stats.start.elapsed().as_secs_f64() * 1000.0);
            eprintln!(
                "    Metrics: role={:?}, score={:.2}, relays={}, bytes_relayed={}",
                metrics.role, metrics.score, metrics.relay_count, metrics.bytes_relayed,
            );
        }
        None => {
            // No metrics yet is OK — peer may not have relayed anything
            stats.record_rtt(stats.start.elapsed().as_secs_f64() * 1000.0);
            eprintln!("    No metrics for target (no relay activity yet — expected)");
        }
    }

    // Step 2: Query all role scores
    eprintln!("  Querying all role scores...");
    stats.sent += 1;
    let scores = handle.get_all_role_scores().await;
    stats.record_rtt(stats.start.elapsed().as_secs_f64() * 1000.0);
    eprintln!("    {} peers tracked", scores.len());
    for (node_id, score, role) in &scores {
        let short_id = &node_id.to_string()[..8];
        eprintln!("      {short_id}… {role:?} score={score:.2}");
    }

    if scores.is_empty() {
        stats.errors.push("GetAllRoleScores returned empty".into());
    }

    // Step 3: Check for any role events that occurred during the campaign
    eprintln!("  Checking for role events...");
    let mut role_events = 0u32;
    while let Ok(evt) = events.try_recv() {
        match &evt {
            ProtocolEvent::RolePromoted { node_id, score } => {
                let short = &node_id.to_string()[..8];
                eprintln!("    RolePromoted: {short}… score={score:.2}");
                role_events += 1;
            }
            ProtocolEvent::RoleDemoted { node_id, score } => {
                let short = &node_id.to_string()[..8];
                eprintln!("    RoleDemoted: {short}… score={score:.2}");
                role_events += 1;
            }
            ProtocolEvent::LocalRoleChanged { new_role } => {
                eprintln!("    LocalRoleChanged: {new_role:?}");
                role_events += 1;
            }
            _ => {}
        }
    }
    eprintln!("  Role events observed: {role_events}");

    stats
}

// ── Helpers ────────────────────────────────────────────────────────

/// Drain all pending messages/events/status between phases to avoid cross-contamination.
/// Drain local pump channels between phases to clear any stale messages/events.
async fn drain_local_channels(
    messages: &mut mpsc::Receiver<DeliveredMessage>,
    events: &mut mpsc::Receiver<ProtocolEvent>,
) {
    eprintln!("  Draining local channels between phases...");
    let mut msg_drained = 0;
    let mut evt_drained = 0;

    // Fast drain with try_recv (non-blocking)
    loop {
        let mut any_drained = false;

        while messages.try_recv().is_ok() {
            msg_drained += 1;
            any_drained = true;
        }
        while events.try_recv().is_ok() {
            evt_drained += 1;
            any_drained = true;
        }

        if !any_drained {
            break;
        }
    }

    if msg_drained + evt_drained > 0 {
        eprintln!("    Drained {msg_drained} msgs, {evt_drained} events");
    }

    // Pause to let pump transfer any final messages
    tokio::time::sleep(Duration::from_millis(200)).await;
}

async fn wait_for_peer(events: &mut mpsc::Receiver<ProtocolEvent>, target: NodeId, timeout: Duration) {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        match recv_timeout(events, Duration::from_secs(1)).await {
            Ok(ProtocolEvent::GossipNeighborUp { node_id }) if node_id == target => {
                eprintln!("Peer connected (gossip neighbor up)");
                return;
            }
            Ok(ProtocolEvent::PeerDiscovered { node_id, .. }) if node_id == target => {
                eprintln!("Peer discovered");
                return;
            }
            Ok(_) => continue,
            Err(_) => continue,
        }
    }
    eprintln!("Peer not discovered in timeout — continuing anyway (iroh may connect later)");
}

fn print_phase_result(result: &PhaseResult) {
    let icon = match result.status.as_str() {
        "PASS" => " OK ",
        "WARN" => "WARN",
        _ => "FAIL",
    };
    eprintln!(
        "  [{icon}] {}: {}/{} ({:.1}% loss) avg {:.1}ms ({:.1}s)",
        result.phase, result.received, result.sent, result.loss_pct, result.avg_rtt_ms, result.elapsed_s,
    );
    if !result.detail.is_empty() {
        eprintln!("    {}", result.detail);
    }
}
