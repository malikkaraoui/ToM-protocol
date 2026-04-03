/// tom-chat — TUI demo for the ToM protocol.
///
/// Full-stack demo: iroh QUIC transport + protocol layer (envelope,
/// crypto, routing) + ratatui terminal UI.
use std::io;
use std::net::UdpSocket;
use std::sync::Arc;
use std::time::{Duration, Instant};

use clap::Parser;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::prelude::*;
use ratatui::widgets::*;
use tom_protocol::{
    DeliveredMessage, NodeId, ProtocolEvent, ProtocolRuntime, RuntimeChannels, RuntimeConfig,
    RuntimeHandle,
};
use tom_transport::{TomNode, TomNodeConfig};

// ── UDP Log Sender ──────────────────────────────────────────────────────

/// Sends structured JSON log lines to a central collector via UDP.
struct UdpLogger {
    socket: UdpSocket,
    target: String,
}

impl UdpLogger {
    fn new(target: &str) -> Option<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
        socket.set_nonblocking(true).ok()?;
        Some(Self {
            socket,
            target: target.to_string(),
        })
    }

    fn send(&self, json: &str) {
        let _ = self.socket.send_to(json.as_bytes(), &self.target);
    }
}

/// Shared context for structured log emission (bot mode).
struct BotContext {
    node_label: String,
    node_id: String,
    udp: Option<UdpLogger>,
    handle: RuntimeHandle,
}

impl BotContext {
    fn log_event(&self, event: &str, detail: &str) {
        let snap = self.handle.metrics();
        let phase_str = format!("{}", snap.phase);
        let role_str = format!("{:?}", snap.role_local);

        let json = serde_json::json!({
            "ts": timestamp_ms(),
            "node": self.node_label,
            "node_id": &self.node_id[..8.min(self.node_id.len())],
            "event": event,
            "detail": detail,
            "phase": phase_str,
            "taille_reseau": snap.taille_reseau,
            "number_peers": snap.peers_known,
            "role": role_str,
            "msgs_sent": snap.messages_sent,
            "msgs_recv": snap.messages_received,
            "relayeurs": snap.relayeurs_connus,
            "groups": snap.groups_count,
            "uptime_s": snap.uptime_seconds,
        });
        let line = json.to_string();

        eprintln!("{}", line);
        if let Some(ref udp) = self.udp {
            udp.send(&line);
        }
    }
}

/// Unix timestamp in milliseconds — universal, no timezone ambiguity.
fn timestamp_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64
}

// ── Status HTTP Server ──────────────────────────────────────────────────

/// Spawns a tiny HTTP server that responds to any GET with a JSON status page.
/// No dependency on hyper — raw TCP + minimal HTTP response.
fn spawn_status_server(port: u16, handle: RuntimeHandle, node_label: String) {
    tokio::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("{{\"event\":\"erreur_status_server\",\"detail\":\"{}\"}}", e);
                return;
            }
        };
        eprintln!("{{\"event\":\"status_server_demarre\",\"detail\":\"port={}\"}}", port);

        loop {
            let Ok((mut stream, _)) = listener.accept().await else { continue };
            let handle = handle.clone();
            let label = node_label.clone();

            tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};

                // Read request (we don't care about the content, just drain it)
                let mut buf = [0u8; 1024];
                let _ = stream.read(&mut buf).await;

                // Build JSON status
                let snap = handle.metrics();
                let peers = handle.connected_peers().await;
                let groups = handle.groups().await;

                let peers_json: Vec<String> = peers.iter().map(|p| {
                    let s = p.to_string();
                    format!("\"{}\"", &s[..8.min(s.len())])
                }).collect();

                let groups_json: Vec<String> = groups.iter().map(|g| {
                    format!("{{\"nom\":\"{}\",\"membres\":{}}}", g.name, g.members.len())
                }).collect();

                let body = format!(
                    concat!(
                        "{{",
                        "\"node\":\"{label}\",",
                        "\"node_id\":\"{node_id}\",",
                        "\"phase\":\"{phase}\",",
                        "\"taille_reseau\":{taille},",
                        "\"role\":\"{role:?}\",",
                        "\"relayeurs\":{relayeurs},",
                        "\"pairs_connectes\":[{peers}],",
                        "\"groupes\":[{groups}],",
                        "\"messages_envoyes\":{sent},",
                        "\"messages_recus\":{recv},",
                        "\"messages_echoues\":{failed},",
                        "\"uptime_secondes\":{uptime}",
                        "}}"
                    ),
                    label = label,
                    node_id = handle.local_id(),
                    phase = snap.phase,
                    taille = snap.taille_reseau,
                    role = snap.role_local,
                    relayeurs = snap.relayeurs_connus,
                    peers = peers_json.join(","),
                    groups = groups_json.join(","),
                    sent = snap.messages_sent,
                    recv = snap.messages_received,
                    failed = snap.messages_failed,
                    uptime = snap.uptime_seconds,
                );

                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );

                let _ = stream.write_all(response.as_bytes()).await;
            });
        }
    });
}

// ── CLI ─────────────────────────────────────────────────────────────────

/// tom-chat — TUI chat demo for the ToM protocol.
#[derive(Parser, Debug)]
#[command(name = "tom-chat", version)]
struct Cli {
    /// Peer node ID to connect to (gossip bootstrap).
    peer: Option<String>,

    /// Username for gossip discovery.
    #[arg(long, default_value = "anonymous")]
    username: String,

    /// Headless bot mode — auto-responds to messages.
    #[arg(long)]
    bot: bool,

    /// Gossip bootstrap peer (alternative to positional arg, can repeat).
    #[arg(long = "bootstrap", value_name = "NODE_ID")]
    bootstrap_peers: Vec<String>,

    // ── Observer options ──

    /// Enable transport relay discovery (observer receives RelayReadyAnnounce via gossip).
    #[arg(long)]
    relay_discovery: bool,

    /// Relay registry TTL in seconds (how long a discovered relay stays valid).
    #[arg(long, value_name = "SECS")]
    relay_ttl: Option<u64>,

    // ── Publisher / self-relay options ──

    /// Start as self-relay: embedded relay on 0.0.0.0:3340 + publish via gossip.
    /// Shorthand for --embedded-relay --embedded-relay-publish --embedded-relay-bind 0.0.0.0:3340.
    #[arg(long)]
    self_relay: bool,

    /// Enable embedded relay server on this node.
    #[arg(long)]
    embedded_relay: bool,

    /// Enable publication of RelayReadyAnnounce via gossip (requires --embedded-relay or --self-relay).
    #[arg(long, requires = "embedded_relay")]
    embedded_relay_publish: bool,

    /// Republication interval in seconds for RelayReadyAnnounce (publisher option, requires --embedded-relay-publish).
    /// Default: relay_ttl / 2.
    #[arg(long, value_name = "SECS", requires = "embedded_relay_publish")]
    relay_publish_interval: Option<u64>,

    /// Bind address for the embedded relay (default: [::]:0 = dual-stack, all interfaces).
    /// Use 127.0.0.1:0 for localhost-only, or a specific IP:PORT.
    #[arg(long, value_name = "ADDR", requires = "embedded_relay")]
    embedded_relay_bind: Option<std::net::SocketAddr>,

    /// Advertised IP for the embedded relay URL (overrides auto-detection).
    /// Use when auto-detection picks the wrong interface (e.g. VPN, Docker).
    #[arg(long, value_name = "IP", requires = "embedded_relay")]
    embedded_relay_advertise: Option<std::net::IpAddr>,

    // ── Bot ping options ──

    /// In bot mode, send a ping message to the first discovered peer every N seconds.
    /// Proves message exchange without human intervention (requires --bot).
    #[arg(long, value_name = "SECS", requires = "bot")]
    bot_ping: Option<u64>,

    // ── Observability options ──

    /// Human-readable label for this node (shown in logs and status).
    #[arg(long, default_value = "unnamed")]
    node_label: String,

    /// UDP host:port for centralized log collection (e.g. "192.168.1.10:9999").
    /// Logs are sent as JSON lines over UDP in addition to stderr.
    #[arg(long, value_name = "HOST:PORT")]
    log_udp: Option<String>,

    /// HTTP port for local status page (e.g. 8080).
    /// Exposes a JSON endpoint showing node identity, phase, peers, roles, metrics.
    #[arg(long, value_name = "PORT")]
    status_port: Option<u16>,
}

// ── App State ────────────────────────────────────────────────────────────

struct App {
    /// Our node identity.
    local_id: NodeId,
    /// Chat messages (timestamp, from_label, text).
    messages: Vec<ChatMessage>,
    /// Current input text.
    input: String,
    /// Connected peer (if any).
    peer_id: Option<NodeId>,
    /// Status line.
    status: String,
    /// Should quit.
    quit: bool,
    /// Scroll offset for messages.
    scroll: u16,
    /// Our short ID for display.
    short_id: String,
    /// Total messages sent/received.
    stats: Stats,
}

struct ChatMessage {
    timestamp: String,
    from: String,
    text: String,
    is_system: bool,
}

#[derive(Default)]
struct Stats {
    sent: u64,
    received: u64,
}

impl App {
    fn new(local_id: NodeId) -> Self {
        let short_id = short_node_id(&local_id);
        Self {
            local_id,
            messages: vec![],
            input: String::new(),
            peer_id: None,
            status: "Ready — waiting for peer".into(),
            quit: false,
            scroll: 0,
            short_id,
            stats: Stats::default(),
        }
    }

    fn add_system_message(&mut self, text: String) {
        self.messages.push(ChatMessage {
            timestamp: now_hms(),
            from: "system".into(),
            text,
            is_system: true,
        });
        self.scroll_to_bottom();
    }

    fn add_chat_message(&mut self, from: &str, text: String) {
        self.messages.push(ChatMessage {
            timestamp: now_hms(),
            from: from.to_string(),
            text,
            is_system: false,
        });
        self.scroll_to_bottom();
    }

    fn scroll_to_bottom(&mut self) {
        if self.messages.len() > 20 {
            self.scroll = (self.messages.len() as u16).saturating_sub(20);
        }
    }
}

// ── Main ─────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Enable tracing in bot mode — JSON format for machine-readable logs
    if cli.bot {
        let _ = tracing_subscriber::fmt()
            .json()
            .with_env_filter(
                tracing_subscriber::EnvFilter::from_default_env()
            )
            .try_init();
    }

    // ── Expand --self-relay shorthand ──
    let use_embedded_relay = cli.embedded_relay || cli.self_relay;
    let use_embedded_relay_publish = cli.embedded_relay_publish || cli.self_relay;
    let effective_bind = if cli.self_relay && cli.embedded_relay_bind.is_none() {
        Some(std::net::SocketAddr::from(([0, 0, 0, 0], 3340)))
    } else {
        cli.embedded_relay_bind
    };

    // Init transport
    let node = TomNode::bind(TomNodeConfig::new()).await?;
    let local_id = node.id();

    // Build runtime config
    let mut config = RuntimeConfig {
        username: cli.username.clone(),
        enable_transport_relay_discovery: cli.relay_discovery,
        enable_embedded_relay: use_embedded_relay,
        enable_embedded_relay_publication: use_embedded_relay_publish,
        ..Default::default()
    };

    // Override TTL if specified
    if let Some(ttl_secs) = cli.relay_ttl {
        config.relay_registry_ttl = Duration::from_secs(ttl_secs);
        // Adjust default publish interval to ttl/2 unless explicitly set
        if cli.relay_publish_interval.is_none() {
            config.relay_publish_interval = Duration::from_secs(ttl_secs / 2);
        }
    }
    if let Some(interval_secs) = cli.relay_publish_interval {
        config.relay_publish_interval = Duration::from_secs(interval_secs);
    }
    if let Some(bind_addr) = effective_bind {
        config.embedded_relay_bind_addr = bind_addr;
    }
    config.embedded_relay_advertise_addr = cli.embedded_relay_advertise;

    // Gossip bootstrap peers: positional + --bootstrap + env
    if let Some(ref peer_str) = cli.peer {
        if let Ok(peer_id) = peer_str.parse::<NodeId>() {
            config.gossip_bootstrap_peers.push(peer_id);
        }
    }
    for peer_str in &cli.bootstrap_peers {
        if let Ok(peer_id) = peer_str.parse::<NodeId>() {
            if !config.gossip_bootstrap_peers.contains(&peer_id) {
                config.gossip_bootstrap_peers.push(peer_id);
            }
        }
    }
    if let Ok(bootstrap) = std::env::var("TOM_BOOTSTRAP_PEER") {
        if let Ok(peer_id) = bootstrap.parse::<NodeId>() {
            if !config.gossip_bootstrap_peers.contains(&peer_id) {
                config.gossip_bootstrap_peers.push(peer_id);
            }
        }
    }

    // ── Startup summary ──
    let mode = if use_embedded_relay { "publisher" } else if cli.relay_discovery { "observer" } else { "peer" };
    let relay_env = std::env::var("TOM_RELAY_URL").unwrap_or_else(|_| "(none)".into());
    eprintln!("tom-chat v0.1 | {} | user={}", mode, cli.username);
    eprintln!("  node    {}", local_id);
    eprintln!("  relay   {}", relay_env);
    if use_embedded_relay {
        eprintln!("  self-relay  bind={}", effective_bind.map_or("default".into(), |a| a.to_string()));
    }
    if cli.relay_discovery {
        eprintln!("  discovery   relay-ttl={}s", cli.relay_ttl.unwrap_or(600));
    }
    if !config.gossip_bootstrap_peers.is_empty() {
        for bp in &config.gossip_bootstrap_peers {
            eprintln!("  bootstrap   {}", short_node_id(bp));
        }
    }
    if cli.bot {
        eprintln!("  bot         ping={}",
            cli.bot_ping.map_or("off".into(), |s| format!("{}s", s)));
    }
    eprintln!();

    // Start protocol runtime (owns the node, handles routing/crypto/tracking)
    let RuntimeChannels {
        handle,
        mut messages,
        status_changes: _status_changes,
        mut events,
    } = ProtocolRuntime::spawn(node, config);

    // Start status HTTP server if requested
    if let Some(port) = cli.status_port {
        spawn_status_server(port, handle.clone(), cli.node_label.clone());
    }

    if cli.bot {
        let udp = cli.log_udp.as_deref().and_then(UdpLogger::new);
        let ctx = Arc::new(BotContext {
            node_label: cli.node_label.clone(),
            node_id: local_id.to_string(),
            udp,
            handle: handle.clone(),
        });
        ctx.log_event("demarrage", &format!("noeud={} mode=bot", cli.node_label));
        return run_bot(ctx, handle, messages, events, cli.bot_ping).await;
    }

    let mut app = App::new(local_id);
    app.add_system_message(format!("Node started: {}", app.short_id));
    app.add_system_message(format!("Full ID: {}", local_id));

    // If peer arg, connect
    if let Some(ref peer_str) = cli.peer {
        match peer_str.parse::<NodeId>() {
            Ok(peer_id) => {
                app.peer_id = Some(peer_id);
                handle.add_peer(peer_id).await;
                app.status = format!("Connecting to {}...", short_node_id(&peer_id));
                app.add_system_message(format!("Connecting to {}...", short_node_id(&peer_id)));
            }
            Err(e) => {
                app.add_system_message(format!("Invalid peer ID: {}", e));
            }
        }
    } else {
        app.add_system_message("No peer specified. Share your Node ID with a peer.".into());
        app.add_system_message("Or restart with: tom-chat <peer-node-id>".into());
    }

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    stdout.execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    let tick_rate = Duration::from_millis(50);
    let mut last_tick = Instant::now();

    loop {
        // Draw
        terminal.draw(|f| draw_ui(f, &app))?;

        // Handle events
        let timeout = tick_rate.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                        app.quit = true;
                    }
                    KeyCode::Esc => {
                        app.quit = true;
                    }
                    KeyCode::Enter => {
                        if !app.input.is_empty() {
                            let text = app.input.drain(..).collect::<String>();
                            handle_input(&mut app, &text, &handle).await;
                        }
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    KeyCode::Up => {
                        app.scroll = app.scroll.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        app.scroll = app.scroll.saturating_add(1);
                    }
                    KeyCode::Char(c) => {
                        app.input.push(c);
                    }
                    _ => {}
                }
            }
        }

        // Process incoming messages (delivered by protocol runtime — already decrypted + verified)
        while let Ok(msg) = messages.try_recv() {
            handle_incoming(&mut app, &msg);
        }

        // Process protocol events
        while let Ok(evt) = events.try_recv() {
            handle_protocol_event(&mut app, &evt);
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }

        if app.quit {
            handle.shutdown().await;
            break;
        }
    }

    // Cleanup
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;

    eprintln!("\n  Stats: {} sent, {} received", app.stats.sent, app.stats.received);
    Ok(())
}

// ── Input handling ───────────────────────────────────────────────────────

async fn handle_input(app: &mut App, text: &str, handle: &RuntimeHandle) {
    // Commands
    if text.starts_with('/') {
        handle_command(app, text);
        return;
    }

    // Send chat message
    let Some(peer_id) = app.peer_id else {
        app.add_system_message("No peer connected. Use /connect <node-id>".into());
        return;
    };

    // Send via protocol runtime (handles envelope, signing, encryption, relay selection)
    match handle.send_message(peer_id, text.as_bytes().to_vec()).await {
        Ok(()) => {
            app.stats.sent += 1;
            app.add_chat_message(&app.short_id.clone(), text.to_string());
            app.status = format!("Sent to {}", short_node_id(&peer_id));
        }
        Err(e) => {
            app.add_system_message(format!("Send error: {}", e));
        }
    }
}

fn handle_command(app: &mut App, cmd: &str) {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    match parts[0] {
        "/connect" | "/c" => {
            if parts.len() < 2 {
                app.add_system_message("Usage: /connect <node-id>".into());
                return;
            }
            match parts[1].trim().parse::<NodeId>() {
                Ok(peer_id) => {
                    app.peer_id = Some(peer_id);
                    app.status = format!("Connected to {}", short_node_id(&peer_id));
                    app.add_system_message(format!("Peer set: {}", short_node_id(&peer_id)));
                }
                Err(e) => {
                    app.add_system_message(format!("Invalid node ID: {}", e));
                }
            }
        }
        "/id" => {
            app.add_system_message(format!("Your ID: {}", app.local_id));
        }
        "/stats" => {
            app.add_system_message(format!(
                "Sent: {} msgs | Received: {} msgs",
                app.stats.sent, app.stats.received
            ));
        }
        "/clear" => {
            app.messages.clear();
            app.scroll = 0;
        }
        "/help" | "/h" => {
            app.add_system_message("Commands:".into());
            app.add_system_message("  /connect <id>  — set peer to chat with".into());
            app.add_system_message("  /id            — show your node ID".into());
            app.add_system_message("  /stats         — show message stats".into());
            app.add_system_message("  /clear         — clear messages".into());
            app.add_system_message("  /quit          — exit".into());
            app.add_system_message("  Ctrl+C / Esc   — exit".into());
        }
        "/quit" | "/q" => {
            app.quit = true;
        }
        _ => {
            app.add_system_message(format!("Unknown command: {}", parts[0]));
        }
    }
}

// ── Incoming message handling ────────────────────────────────────────────

fn handle_incoming(app: &mut App, msg: &DeliveredMessage) {
    let sig_label = if msg.signature_valid { "verified" } else { "unverified" };
    let enc_label = if msg.was_encrypted { "encrypted" } else { "plain" };

    let from_short = short_node_id(&msg.from);
    let text = String::from_utf8_lossy(&msg.payload);

    app.stats.received += 1;
    app.add_chat_message(
        &from_short,
        format!("{} [{}, {}]", text, sig_label, enc_label),
    );

    // Auto-set peer if not set
    if app.peer_id.is_none() {
        app.peer_id = Some(msg.from);
        app.status = format!("Connected: {}", from_short);
        app.add_system_message(format!("Auto-connected to {}", from_short));
    }
}

// ── Protocol event handling ──────────────────────────────────────────────

fn handle_protocol_event(app: &mut App, event: &ProtocolEvent) {
    match event {
        ProtocolEvent::PeerDiscovered { node_id, username, source } => {
            app.add_system_message(format!(
                "Peer discovered: {} \"{}\" (via {:?})",
                short_node_id(node_id),
                username,
                source
            ));
            // Auto-set peer if not set (discovered via gossip/announce)
            if app.peer_id.is_none() {
                app.peer_id = Some(*node_id);
                app.status = format!("Connected: {} (via {:?})", short_node_id(node_id), source);
                app.add_system_message(format!("Auto-connected to {} via {:?}", short_node_id(node_id), source));
            }
        }
        ProtocolEvent::PeerStale { node_id } => {
            app.add_system_message(format!("Peer stale: {}", short_node_id(node_id)));
        }
        ProtocolEvent::PeerOffline { node_id } => {
            app.add_system_message(format!("Peer offline: {}", short_node_id(node_id)));
        }
        ProtocolEvent::PeerOnline { node_id } => {
            app.add_system_message(format!("Peer online: {}", short_node_id(node_id)));
        }
        ProtocolEvent::PathChanged { event } => {
            app.add_system_message(format!("Path changed: {:?}", event));
        }
        ProtocolEvent::GossipNeighborUp { node_id } => {
            app.add_system_message(format!("Gossip: neighbor up {}", short_node_id(node_id)));
        }
        ProtocolEvent::GossipNeighborDown { node_id } => {
            app.add_system_message(format!("Gossip: neighbor down {}", short_node_id(node_id)));
        }
        ProtocolEvent::Error { description } => {
            app.add_system_message(format!("Error: {}", description));
        }
        // ── Embedded relay events ──
        ProtocolEvent::EmbeddedRelayStarted { url } => {
            app.add_system_message(format!("Embedded relay started: {}", url));
        }
        ProtocolEvent::EmbeddedRelayFailed { error } => {
            app.add_system_message(format!("Embedded relay FAILED: {}", error));
        }
        ProtocolEvent::EmbeddedRelayStopped => {
            app.add_system_message("Embedded relay stopped".into());
        }
        // ── Relay discovery events ──
        ProtocolEvent::RelayReadyReceived { node_id, relay_url } => {
            app.add_system_message(format!(
                "Relay discovered: {} → {}",
                short_node_id(node_id),
                relay_url
            ));
        }
        ProtocolEvent::RelayRegistryExpired { node_id, relay_url } => {
            app.add_system_message(format!(
                "Relay expired: {} → {}",
                short_node_id(node_id),
                relay_url
            ));
        }
        ProtocolEvent::TransportRelayInserted { relay_url } => {
            app.add_system_message(format!("Transport relay added: {}", relay_url));
        }
        ProtocolEvent::TransportRelayRemoved { relay_url } => {
            app.add_system_message(format!("Transport relay removed: {}", relay_url));
        }
        _ => {}
    }
}

// ── UI Drawing ───────────────────────────────────────────────────────────

fn draw_ui(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(5),     // Messages
            Constraint::Length(3),  // Input
            Constraint::Length(1),  // Status
        ])
        .split(f.area());

    // Header
    let peer_info = match &app.peer_id {
        Some(id) => format!(" → {}", short_node_id(id)),
        None => " (no peer)".into(),
    };
    let header = Paragraph::new(format!(" tom-chat  |  You: {}  |  Peer{}", app.short_id, peer_info))
        .style(Style::default().fg(Color::White).bg(Color::DarkGray).bold())
        .block(Block::default());
    f.render_widget(header, chunks[0]);

    // Messages
    let msg_items: Vec<Line> = app
        .messages
        .iter()
        .map(|m| {
            if m.is_system {
                Line::from(vec![
                    Span::styled(
                        format!("[{}] ", m.timestamp),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(&m.text, Style::default().fg(Color::Yellow).italic()),
                ])
            } else {
                let is_self = m.from == app.short_id;
                let name_color = if is_self { Color::Cyan } else { Color::Green };
                Line::from(vec![
                    Span::styled(
                        format!("[{}] ", m.timestamp),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(
                        format!("{}: ", m.from),
                        Style::default().fg(name_color).bold(),
                    ),
                    Span::raw(&m.text),
                ])
            }
        })
        .collect();

    let messages = Paragraph::new(msg_items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Messages ")
                .border_style(Style::default().fg(Color::DarkGray)),
        )
        .scroll((app.scroll, 0))
        .wrap(Wrap { trim: false });
    f.render_widget(messages, chunks[1]);

    // Input
    let input = Paragraph::new(app.input.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Type message (Enter to send, /help for commands) ")
                .border_style(Style::default().fg(Color::Cyan)),
        );
    f.render_widget(input, chunks[2]);

    // Cursor position
    let cursor_x = chunks[2].x + app.input.len() as u16 + 1;
    let cursor_y = chunks[2].y + 1;
    f.set_cursor_position((cursor_x.min(chunks[2].right() - 2), cursor_y));

    // Status
    let status = Paragraph::new(format!(" {} ", app.status))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(status, chunks[3]);
}

// ── Bot Mode ─────────────────────────────────────────────────────────

async fn run_bot(
    ctx: Arc<BotContext>,
    handle: RuntimeHandle,
    mut messages: tokio::sync::mpsc::Receiver<DeliveredMessage>,
    mut events: tokio::sync::mpsc::Receiver<ProtocolEvent>,
    bot_ping_secs: Option<u64>,
) -> anyhow::Result<()> {
    ctx.log_event("bot_start", &format!("id={}", handle.local_id()));

    let mut count = 0u64;
    let mut ping_count = 0u64;
    let mut ping_target: Option<NodeId> = None;

    let ping_duration = bot_ping_secs.map(Duration::from_secs);
    let mut ping_interval = tokio::time::interval(ping_duration.unwrap_or(Duration::from_secs(3600)));
    ping_interval.tick().await;

    loop {
        tokio::select! {
            msg_opt = messages.recv() => {
                let Some(msg) = msg_opt else {
                    ctx.log_event("arret", "canal fermé");
                    break;
                };

                let text = String::from_utf8_lossy(&msg.payload);
                count += 1;

                ctx.log_event("message_recu", &format!(
                    "de={} sig={} contenu={}",
                    short_node_id(&msg.from),
                    if msg.signature_valid { "ok" } else { "bad" },
                    text
                ));

                let reply = format!("recu 5/5 (msg #{})", count);
                match handle.send_message(msg.from, reply.as_bytes().to_vec()).await {
                    Ok(()) => ctx.log_event("reponse_envoyee", &format!("a={}", short_node_id(&msg.from))),
                    Err(e) => ctx.log_event("erreur_envoi", &e.to_string()),
                }
            }
            evt_opt = events.recv() => {
                let Some(evt) = evt_opt else { break; };
                if let Some(target) = select_ping_target(ping_target, &evt) {
                    if let ProtocolEvent::PeerDiscovered { username, .. } = &evt {
                        ctx.log_event("cible_ping", &format!("{} \"{}\"", short_node_id(&target), username));
                    }
                    ping_target = Some(target);
                }
                handle_bot_event(&ctx, &evt);
            }
            _ = ping_interval.tick(), if ping_duration.is_some() && ping_target.is_some() => {
                let target = ping_target.unwrap();
                ping_count += 1;
                let msg = format!("ping #{} from {}", ping_count, short_node_id(&handle.local_id()));
                match handle.send_message(target, msg.as_bytes().to_vec()).await {
                    Ok(()) => ctx.log_event("ping_envoye", &format!("#{} a={}", ping_count, short_node_id(&target))),
                    Err(e) => ctx.log_event("erreur_ping", &e.to_string()),
                }
            }
        }
    }

    Ok(())
}

fn handle_bot_event(ctx: &BotContext, event: &ProtocolEvent) {
    match event {
        ProtocolEvent::PeerDiscovered { node_id, username, source } => {
            ctx.log_event("pair_trouve", &format!("{} \"{}\" via {:?}", short_node_id(node_id), username, source));
        }
        ProtocolEvent::GossipNeighborUp { node_id } => {
            ctx.log_event("voisin_connecte", &short_node_id(node_id));
        }
        ProtocolEvent::GossipNeighborDown { node_id } => {
            ctx.log_event("voisin_deconnecte", &short_node_id(node_id));
        }
        ProtocolEvent::EmbeddedRelayStarted { url } => {
            ctx.log_event("relayeur_demarre", &url.to_string());
        }
        ProtocolEvent::EmbeddedRelayFailed { error } => {
            ctx.log_event("relayeur_echec", error);
        }
        ProtocolEvent::EmbeddedRelayStopped => {
            ctx.log_event("relayeur_arrete", "");
        }
        ProtocolEvent::RelayReadyReceived { node_id, relay_url } => {
            ctx.log_event("relayeur_decouvert", &format!("{} → {}", short_node_id(node_id), relay_url));
        }
        ProtocolEvent::TransportRelayInserted { relay_url } => {
            ctx.log_event("relayeur_ajoute", &relay_url.to_string());
        }
        ProtocolEvent::TransportRelayRemoved { relay_url } => {
            ctx.log_event("relayeur_retire", &relay_url.to_string());
        }
        ProtocolEvent::PathChanged { event } => {
            ctx.log_event("chemin_change", &format!("{:?}", event));
        }
        ProtocolEvent::RolePromoted { node_id, score } => {
            ctx.log_event("role_promu_relayeur", &format!("{} score={:.1}", short_node_id(node_id), score));
        }
        ProtocolEvent::RoleDemoted { node_id, score } => {
            ctx.log_event("role_retro_participant", &format!("{} score={:.1}", short_node_id(node_id), score));
        }
        ProtocolEvent::GroupCreated { group } => {
            ctx.log_event("groupe_cree", &group.name);
        }
        ProtocolEvent::GroupJoined { group_name, .. } => {
            ctx.log_event("groupe_rejoint", group_name);
        }
        ProtocolEvent::GroupMemberJoined { .. } => {
            ctx.log_event("membre_rejoint_groupe", "");
        }
        ProtocolEvent::GroupHubMigrated { new_hub_id, .. } => {
            ctx.log_event("responsable_groupe_change", &short_node_id(new_hub_id));
        }
        _ => {}
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────

/// Evaluate whether a `PeerDiscovered` event should set the bot-ping target.
///
/// Returns `Some(node_id)` if:
/// - no target is set yet (`current_target` is `None`)
/// - the discovered peer has a non-empty username (real ToM peer, not anonymous n0/Pkarr)
///
/// This is the regression-critical logic: anonymous peers MUST be skipped.
fn select_ping_target(
    current_target: Option<NodeId>,
    event: &ProtocolEvent,
) -> Option<NodeId> {
    if current_target.is_some() {
        return None; // already locked
    }
    if let ProtocolEvent::PeerDiscovered { node_id, username, .. } = event {
        if !username.is_empty() {
            return Some(*node_id);
        }
    }
    None
}

fn short_node_id(id: &NodeId) -> String {
    let s = id.to_string();
    if s.len() > 8 {
        format!("{}…", &s[..8])
    } else {
        s
    }
}

fn now_hms() -> String {
    chrono_lite_hms()
}

/// Minimal HH:MM:SS without pulling in chrono.
fn chrono_lite_hms() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let h = (secs / 3600) % 24;
    let m = (secs / 60) % 60;
    let s = secs % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    fn try_parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(args)
    }

    #[test]
    fn cli_defaults() {
        let cli = try_parse(&["tom-chat"]).unwrap();
        assert_eq!(cli.username, "anonymous");
        assert!(!cli.bot);
        assert!(!cli.relay_discovery);
        assert!(!cli.embedded_relay);
        assert!(!cli.embedded_relay_publish);
        assert!(!cli.self_relay);
        assert!(cli.relay_ttl.is_none());
        assert!(cli.relay_publish_interval.is_none());
        assert!(cli.peer.is_none());
        assert!(cli.bootstrap_peers.is_empty());
    }

    #[test]
    fn cli_observer_mode() {
        let cli = try_parse(&[
            "tom-chat", "--username", "obs", "--relay-discovery", "--relay-ttl", "30",
        ]).unwrap();
        assert_eq!(cli.username, "obs");
        assert!(cli.relay_discovery);
        assert_eq!(cli.relay_ttl, Some(30));
    }

    #[test]
    fn cli_publisher_mode() {
        let cli = try_parse(&[
            "tom-chat", "--embedded-relay", "--embedded-relay-publish",
            "--relay-publish-interval", "5",
        ]).unwrap();
        assert!(cli.embedded_relay);
        assert!(cli.embedded_relay_publish);
        assert_eq!(cli.relay_publish_interval, Some(5));
    }

    #[test]
    fn cli_publish_requires_embedded_relay() {
        let err = try_parse(&["tom-chat", "--embedded-relay-publish"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn cli_publish_interval_requires_publish() {
        let err = try_parse(&["tom-chat", "--relay-publish-interval", "5"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    #[test]
    fn cli_peer_positional() {
        let cli = try_parse(&["tom-chat", "abc123"]).unwrap();
        assert_eq!(cli.peer.as_deref(), Some("abc123"));
    }

    #[test]
    fn cli_bot_mode() {
        let cli = try_parse(&["tom-chat", "--bot"]).unwrap();
        assert!(cli.bot);
        assert!(cli.bot_ping.is_none());
    }

    #[test]
    fn cli_bot_ping() {
        let cli = try_parse(&["tom-chat", "--bot", "--bot-ping", "5"]).unwrap();
        assert!(cli.bot);
        assert_eq!(cli.bot_ping, Some(5));
    }

    #[test]
    fn cli_bot_ping_requires_bot() {
        let err = try_parse(&["tom-chat", "--bot-ping", "5"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::MissingRequiredArgument);
    }

    // ── --self-relay shorthand tests ────────────────────────────────────

    #[test]
    fn cli_self_relay_sets_embedded_flags() {
        let cli = try_parse(&["tom-chat", "--self-relay"]).unwrap();
        assert!(cli.self_relay);
        // --self-relay doesn't set embedded_relay directly (expanded at runtime)
        assert!(!cli.embedded_relay);
    }

    #[test]
    fn cli_self_relay_with_custom_bind() {
        // --embedded-relay-bind requires --embedded-relay, but --self-relay should
        // allow the user to override bind without --embedded-relay explicit
        let cli = try_parse(&["tom-chat", "--self-relay", "--embedded-relay", "--embedded-relay-bind", "127.0.0.1:4000"]).unwrap();
        assert!(cli.self_relay);
        assert_eq!(cli.embedded_relay_bind, Some("127.0.0.1:4000".parse().unwrap()));
    }

    // ── --bootstrap tests ────────────────────────────────────────────────

    #[test]
    fn cli_bootstrap_single() {
        let cli = try_parse(&["tom-chat", "--bootstrap", "abc123"]).unwrap();
        assert_eq!(cli.bootstrap_peers, vec!["abc123"]);
    }

    #[test]
    fn cli_bootstrap_multiple() {
        let cli = try_parse(&["tom-chat", "--bootstrap", "aaa", "--bootstrap", "bbb"]).unwrap();
        assert_eq!(cli.bootstrap_peers, vec!["aaa", "bbb"]);
    }

    #[test]
    fn cli_bootstrap_and_positional() {
        let cli = try_parse(&["tom-chat", "positional_peer", "--bootstrap", "named_peer"]).unwrap();
        assert_eq!(cli.peer.as_deref(), Some("positional_peer"));
        assert_eq!(cli.bootstrap_peers, vec!["named_peer"]);
    }

    // ── Bot-ping target selection regression tests ──────────────────────

    fn make_peer_discovered(node_id: NodeId, username: &str) -> ProtocolEvent {
        ProtocolEvent::PeerDiscovered {
            node_id,
            username: username.to_string(),
            source: tom_protocol::DiscoverySource::Gossip,
        }
    }

    fn random_node_id() -> NodeId {
        let mut rng = rand::rng();
        let secret = tom_base::SecretKey::generate(&mut rng);
        // PublicKey → hex string → NodeId via FromStr
        secret.public().to_string().parse().unwrap()
    }

    /// REGRESSION: anonymous n0 peer (empty username) must NOT be selected as ping target.
    #[test]
    fn bot_ping_skips_anonymous_peer() {
        let anon_id = random_node_id();
        let event = make_peer_discovered(anon_id, "");
        assert!(
            select_ping_target(None, &event).is_none(),
            "anonymous peer with empty username must be skipped"
        );
    }

    /// Named peer (non-empty username) MUST be selected as ping target.
    #[test]
    fn bot_ping_selects_named_peer() {
        let named_id = random_node_id();
        let event = make_peer_discovered(named_id, "nas-publisher");
        let result = select_ping_target(None, &event);
        assert_eq!(result, Some(named_id), "named peer must be selected");
    }

    /// Once a target is locked, subsequent peers (even named) must be ignored.
    #[test]
    fn bot_ping_target_locked_after_first() {
        let first = random_node_id();
        let second = random_node_id();
        let event = make_peer_discovered(second, "mac-obs2");
        assert!(
            select_ping_target(Some(first), &event).is_none(),
            "must not override already-locked target"
        );
    }

    /// REGRESSION: anonymous peer arrives first, then named peer — named peer must win.
    #[test]
    fn bot_ping_anonymous_then_named() {
        let anon_id = random_node_id();
        let named_id = random_node_id();

        // Simulate event sequence: anonymous first
        let anon_event = make_peer_discovered(anon_id, "");
        let mut target = select_ping_target(None, &anon_event);
        assert!(target.is_none(), "anonymous must be skipped");

        // Then named peer arrives
        let named_event = make_peer_discovered(named_id, "nas-publisher");
        target = select_ping_target(None, &named_event);
        assert_eq!(target, Some(named_id), "named peer must be selected after anonymous was skipped");
    }

    /// Non-PeerDiscovered events must be ignored.
    #[test]
    fn bot_ping_ignores_non_discovery_events() {
        let event = ProtocolEvent::PeerStale { node_id: random_node_id() };
        assert!(select_ping_target(None, &event).is_none());
    }
}
