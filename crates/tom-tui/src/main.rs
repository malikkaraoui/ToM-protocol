/// tom-chat — TUI demo for the ToM protocol.
///
/// Full-stack demo: iroh QUIC transport + protocol layer (envelope,
/// crypto, routing) + ratatui terminal UI.
///
/// Usage:
///   tom-chat                     # Start fresh node (TUI)
///   tom-chat <peer-node-id>      # Start and connect to peer (TUI)
///   tom-chat --bot               # Headless bot — auto-responds to messages
use std::io;
use std::time::{Duration, Instant};

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
    // Parse CLI args
    let args: Vec<String> = std::env::args().collect();
    let bot_mode = args.iter().any(|a| a == "--bot");
    let peer_arg = args.get(1).filter(|a| !a.starts_with('-')).cloned();

    // Init transport
    let node = TomNode::bind(TomNodeConfig::new()).await?;
    let local_id = node.id();

    // Print node info to stderr (visible after TUI exits)
    eprintln!("╭─────────────────────────────────────────────╮");
    eprintln!("│  tom-chat v0.1                              │");
    eprintln!("├─────────────────────────────────────────────┤");
    eprintln!("│  Node ID: {}..  │", &local_id.to_string()[..40]);
    eprintln!("│  Short:   {}                          │", short_node_id(&local_id));
    eprintln!("╰─────────────────────────────────────────────╯");

    // Build runtime config — pass CLI peer as gossip bootstrap
    let mut config = RuntimeConfig::default();
    if let Some(ref peer_str) = peer_arg {
        if let Ok(peer_id) = peer_str.parse::<NodeId>() {
            config.gossip_bootstrap_peers = vec![peer_id];
        }
    }

    // Start protocol runtime (owns the node, handles routing/crypto/tracking)
    let RuntimeChannels {
        handle,
        mut messages,
        status_changes: _status_changes,
        mut events,
    } = ProtocolRuntime::spawn(node, config);

    if bot_mode {
        return run_bot(handle, messages).await;
    }

    let mut app = App::new(local_id);
    app.add_system_message(format!("Node started: {}", app.short_id));
    app.add_system_message(format!("Full ID: {}", local_id));

    // If peer arg, connect
    if let Some(ref peer_str) = peer_arg {
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
    handle: RuntimeHandle,
    mut messages: tokio::sync::mpsc::Receiver<DeliveredMessage>,
) -> anyhow::Result<()> {
    println!("[bot] Running in bot mode — auto-responding via ProtocolRuntime");
    println!("[bot] Node ID: {}", handle.local_id());
    println!("[bot] Ctrl+C to stop\n");

    let mut count = 0u64;

    loop {
        let Some(msg) = messages.recv().await else {
            println!("[bot] runtime channel closed, shutting down");
            break;
        };

        let sig_label = if msg.signature_valid { "ok" } else { "bad" };
        let text = String::from_utf8_lossy(&msg.payload);
        count += 1;

        println!(
            "[bot] #{} from {} | sig={} | \"{}\"",
            count,
            short_node_id(&msg.from),
            sig_label,
            text
        );

        // Auto-reply via runtime (handles signing, encryption, relay selection)
        let reply = format!("recu 5/5 malik (msg #{})", count);
        match handle.send_message(msg.from, reply.as_bytes().to_vec()).await {
            Ok(()) => println!("[bot] replied: \"{}\"", reply),
            Err(e) => println!("[bot] send error: {}", e),
        }
    }

    Ok(())
}

// ── Helpers ──────────────────────────────────────────────────────────────

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
