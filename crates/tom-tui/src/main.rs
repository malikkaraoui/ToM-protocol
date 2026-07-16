/// tom-chat — TUI demo for the ToM protocol.
///
/// Full-stack demo: iroh QUIC transport + protocol layer (envelope,
/// crypto, routing) + ratatui terminal UI.
use std::collections::{VecDeque, HashMap};
use std::io;
use std::net::UdpSocket;
use std::sync::{Arc, Mutex};
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

/// Per-peer path information
#[derive(Debug, Clone)]
struct PathInfo {
    kind: String,
    rtt_ms: u64,
    addr: String,
}

/// Type alias for per-peer path info: node_id → PathInfo
type PathsByPeerMap = HashMap<String, PathInfo>;

/// Hard cap for per-peer path tracking (same spirit as the FFI cache bound).
const MAX_PATH_ENTRIES: usize = 2048;

/// Track path changes per peer (bounded) and purge entries when a peer leaves.
fn track_path_event(paths_by_peer: &Arc<Mutex<PathsByPeerMap>>, evt: &ProtocolEvent) {
    match evt {
        ProtocolEvent::PathChanged { event } => {
            let mut pbp = paths_by_peer.lock().unwrap();
            let node_id = event.remote.to_string();
            // Bounded insert: ignore if at capacity and key is new
            if pbp.contains_key(&node_id) || pbp.len() < MAX_PATH_ENTRIES {
                pbp.insert(
                    node_id,
                    PathInfo {
                        kind: format!("{}", event.kind),
                        rtt_ms: event.rtt.as_millis() as u64,
                        addr: event.addr.clone(),
                    },
                );
            }
        }
        ProtocolEvent::PeerOffline { node_id } | ProtocolEvent::PeerStale { node_id } => {
            paths_by_peer.lock().unwrap().remove(&node_id.to_string());
        }
        _ => {}
    }
}

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

// ── Inbox partagé (observabilité des messages reçus, pour tests R13) ─────

/// Un message reçu, capturé pour interrogation via l'endpoint /inbox.
#[derive(Clone)]
struct InboxEntry {
    ts: u64,
    from: String,
    text: String,
}

/// Journal borné des messages reçus, partagé entre run_bot et le control server.
type Inbox = Arc<Mutex<VecDeque<InboxEntry>>>;

/// Capacité max de l'inbox (anti-OOM ; on ne garde que les plus récents).
const INBOX_MAX: usize = 1000;

/// Échappe une chaîne pour inclusion dans une valeur JSON.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// Shared context for structured log emission (bot mode).
struct BotContext {
    node_label: String,
    node_id: String,
    appareil: String,
    udp: Option<UdpLogger>,
    handle: RuntimeHandle,
}

impl BotContext {
    fn log_event(&self, event: &str, detail: &str) {
        self.log_event_sourced(event, detail, None);
    }

    fn log_event_sourced(&self, event: &str, detail: &str, source_amorcage: Option<&str>) {
        let snap = self.handle.metrics();
        let phase_str = format!("{}", snap.phase);
        let role_str = format!("{:?}", snap.role_local);

        let mut json = serde_json::json!({
            "ts": timestamp_ms(),
            "node": self.node_label,
            "node_id": &self.node_id[..8.min(self.node_id.len())],
            "appareil": self.appareil,
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
        if let Some(src) = source_amorcage {
            json["source_amorcage"] = serde_json::Value::String(src.to_string());
        }
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
fn spawn_status_server(
    port: u16,
    handle: RuntimeHandle,
    node_label: String,
    paths_by_peer: Arc<Mutex<PathsByPeerMap>>,
) {
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
            let paths_by_peer = paths_by_peer.clone();

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

                let platform = if cfg!(target_os = "macos") { "macos" }
                    else if cfg!(target_os = "linux") { "linux" }
                    else if cfg!(target_os = "windows") { "windows" }
                    else { "unknown" };
                let relay_url = std::env::var("TOM_RELAY_URL").unwrap_or_default();

                // Build paths_by_peer JSON array using serde_json for correct escaping
                let paths_json = {
                    let pbp = paths_by_peer.lock().unwrap();
                    let paths_list: Vec<serde_json::Value> = pbp.iter().map(|(node_id, info)| {
                        serde_json::json!({
                            "node_id": node_id,
                            "kind": &info.kind,
                            "rtt_ms": info.rtt_ms,
                            "addr": &info.addr
                        })
                    }).collect();
                    serde_json::Value::Array(paths_list).to_string()
                };

                let body = format!(
                    concat!(
                        "{{",
                        "\"schema_version\":1,",
                        "\"node\":\"{label}\",",
                        "\"node_id\":\"{node_id}\",",
                        "\"platform\":\"{platform}\",",
                        "\"relay_url_active\":\"{relay}\",",
                        "\"phase\":\"{phase}\",",
                        "\"taille_reseau\":{taille},",
                        "\"role\":\"{role:?}\",",
                        "\"relayeurs\":{relayeurs},",
                        "\"pairs_connectes\":[{peers}],",
                        "\"groupes\":[{groups}],",
                        "\"paths_by_peer\":{paths},",
                        "\"messages_envoyes\":{sent},",
                        "\"messages_recus\":{recv},",
                        "\"messages_echoues\":{failed},",
                        "\"uptime_secondes\":{uptime}",
                        "}}"
                    ),
                    label = label,
                    node_id = handle.local_id(),
                    platform = platform,
                    relay = relay_url,
                    phase = snap.phase,
                    taille = snap.taille_reseau,
                    role = snap.role_local,
                    relayeurs = snap.relayeurs_connus,
                    peers = peers_json.join(","),
                    groups = groups_json.join(","),
                    paths = paths_json,
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

/// Serveur de CONTRÔLE HTTP (TCP brut, sans dépendance) : permet de piloter
/// le nœud à distance — envoyer des paquets de taille arbitraire, créer des
/// groupes, arrêter le nœud. Conçu pour l'orchestration de tests automatisée.
///
/// Routes (toutes en query-string, réponses JSON) :
///   GET  /peers                          → node_ids complets des pairs connectés
///   GET  /groups                         → groupes (id complet + nb membres)
///   POST /send?to=<id>&size=<n>          → envoie n octets à un pair
///   POST /group/create?name=<n>&members=<id,id>  → crée un groupe (hub = soi)
///   POST /group/send?group=<gid>&size=<n>        → message de n octets au groupe
///   POST /stop                           → arrêt propre du nœud (process exit)
fn spawn_control_server(port: u16, handle: RuntimeHandle, inbox: Inbox) {
    tokio::spawn(async move {
        let listener = match tokio::net::TcpListener::bind(format!("0.0.0.0:{port}")).await {
            Ok(l) => l,
            Err(e) => {
                eprintln!("{{\"event\":\"erreur_control_server\",\"detail\":\"{e}\"}}");
                return;
            }
        };
        eprintln!("{{\"event\":\"control_server_demarre\",\"detail\":\"port={port}\"}}");

        loop {
            let Ok((mut stream, _)) = listener.accept().await else { continue };
            let handle = handle.clone();
            let inbox = inbox.clone();
            tokio::spawn(async move {
                use tokio::io::{AsyncReadExt, AsyncWriteExt};
                let mut buf = vec![0u8; 8192];
                let n = stream.read(&mut buf).await.unwrap_or(0);
                let req = String::from_utf8_lossy(&buf[..n]);
                let line = req.lines().next().unwrap_or("");
                // "METHOD /path?query HTTP/1.1"
                let mut parts = line.split_whitespace();
                let _method = parts.next().unwrap_or("");
                let target = parts.next().unwrap_or("/");
                let (path, query) = target.split_once('?').unwrap_or((target, ""));
                let q = |k: &str| -> Option<String> {
                    query.split('&').find_map(|kv| {
                        let (kk, vv) = kv.split_once('=')?;
                        if kk == k { Some(vv.to_string()) } else { None }
                    })
                };

                let body: String = match path {
                    "/peers" => {
                        let peers = handle.connected_peers().await;
                        let ids: Vec<String> =
                            peers.iter().map(|p| format!("\"{p}\"")).collect();
                        format!("{{\"pairs\":[{}]}}", ids.join(","))
                    }
                    "/groups" => {
                        let groups = handle.groups().await;
                        let gs: Vec<String> = groups
                            .iter()
                            .map(|g| {
                                format!(
                                    "{{\"id\":\"{}\",\"nom\":\"{}\",\"membres\":{}}}",
                                    g.group_id, g.name, g.members.len()
                                )
                            })
                            .collect();
                        format!("{{\"groupes\":[{}]}}", gs.join(","))
                    }
                    "/send" => {
                        // ?to=<id>&size=<n>&count=<k> — envoie k messages de n octets.
                        let size: usize = q("size").and_then(|s| s.parse().ok()).unwrap_or(1024);
                        let count: usize = q("count").and_then(|s| s.parse().ok()).unwrap_or(1);
                        match q("to").and_then(|s| s.parse::<NodeId>().ok()) {
                            Some(to) => {
                                let mut ok = 0usize;
                                let mut err: Option<String> = None;
                                for seq in 0..count {
                                    let mut payload = format!("CTRL:{size}:{seq}:").into_bytes();
                                    payload.resize(size.max(payload.len()), b'A');
                                    match handle.send_message(to, payload).await {
                                        Ok(()) => ok += 1,
                                        Err(e) => { err = Some(e.to_string()); break; }
                                    }
                                }
                                match err {
                                    None => format!(
                                        "{{\"ok\":true,\"envoyes\":{ok},\"taille\":{size},\"vers\":\"{to}\"}}"
                                    ),
                                    Some(e) => format!("{{\"ok\":false,\"envoyes\":{ok},\"erreur\":\"{e}\"}}"),
                                }
                            }
                            None => "{\"ok\":false,\"erreur\":\"param 'to' invalide\"}".into(),
                        }
                    }
                    "/sendall" => {
                        // ?size=<n>&count=<k> — envoie à TOUS les pairs connectés.
                        let size: usize = q("size").and_then(|s| s.parse().ok()).unwrap_or(1024);
                        let count: usize = q("count").and_then(|s| s.parse().ok()).unwrap_or(1);
                        let peers = handle.connected_peers().await;
                        let mut sent = 0usize;
                        for to in &peers {
                            for seq in 0..count {
                                let mut payload = format!("CTRL:{size}:{seq}:").into_bytes();
                                payload.resize(size.max(payload.len()), b'A');
                                if handle.send_message(*to, payload).await.is_ok() {
                                    sent += 1;
                                }
                            }
                        }
                        format!(
                            "{{\"ok\":true,\"envoyes\":{sent},\"pairs\":{},\"taille\":{size}}}",
                            peers.len()
                        )
                    }
                    "/metrics" => {
                        // Relevé compact : compteurs du nœud (pour scoreboard distant).
                        let m = handle.metrics();
                        format!(
                            "{{\"envoyes\":{},\"recus\":{},\"echoues\":{},\"pairs\":{},\"uptime\":{}}}",
                            m.messages_sent, m.messages_received, m.messages_failed,
                            handle.connected_peers().await.len(), m.uptime_seconds
                        )
                    }
                    "/group/create" => {
                        let name = q("name").unwrap_or_else(|| "grp".into());
                        let members: Vec<NodeId> = q("members")
                            .unwrap_or_default()
                            .split(',')
                            .filter_map(|s| s.parse::<NodeId>().ok())
                            .collect();
                        let hub = handle.local_id();
                        match handle.create_group(name.clone(), hub, members).await {
                            Ok(()) => format!("{{\"ok\":true,\"groupe_cree\":\"{name}\"}}"),
                            Err(e) => format!("{{\"ok\":false,\"erreur\":\"{e}\"}}"),
                        }
                    }
                    "/group/send" => {
                        let size: usize = q("size").and_then(|s| s.parse().ok()).unwrap_or(1024);
                        match q("group") {
                            Some(gid) => {
                                let text = format!("CTRL:{size}:")
                                    + &"A".repeat(size.saturating_sub(12));
                                match handle
                                    .send_group_message(gid.clone().into(), text)
                                    .await
                                {
                                    Ok(()) => format!(
                                        "{{\"ok\":true,\"envoye\":{size},\"groupe\":\"{gid}\"}}"
                                    ),
                                    Err(e) => format!("{{\"ok\":false,\"erreur\":\"{e}\"}}"),
                                }
                            }
                            None => "{\"ok\":false,\"erreur\":\"param 'group' manquant\"}".into(),
                        }
                    }
                    "/inbox" => {
                        // ?contains=<substr>&limit=<n> — messages reçus (capturés en bot mode).
                        // Sert à valider le rattrapage offline R13 par contenu.
                        let contains = q("contains");
                        let limit: usize = q("limit").and_then(|s| s.parse().ok()).unwrap_or(200);
                        let snapshot: Vec<InboxEntry> = {
                            let guard = inbox.lock().unwrap();
                            guard.iter().rev().cloned().collect()
                        };
                        let filtered: Vec<String> = snapshot
                            .iter()
                            .filter(|e| {
                                contains.as_deref().map(|c| e.text.contains(c)).unwrap_or(true)
                            })
                            .take(limit)
                            .map(|e| {
                                format!(
                                    "{{\"ts\":{},\"de\":\"{}\",\"texte\":\"{}\"}}",
                                    e.ts,
                                    json_escape(&e.from),
                                    json_escape(&e.text.chars().take(200).collect::<String>())
                                )
                            })
                            .collect();
                        format!(
                            "{{\"total\":{},\"messages\":[{}]}}",
                            filtered.len(),
                            filtered.join(",")
                        )
                    }
                    "/invites" => {
                        let invites = handle.pending_invites().await;
                        let is: Vec<String> = invites
                            .iter()
                            .map(|i| {
                                format!(
                                    "{{\"group_id\":\"{}\",\"nom\":\"{}\",\"inviteur\":\"{}\"}}",
                                    json_escape(i.group_id.as_ref()),
                                    json_escape(&i.group_name),
                                    json_escape(&i.inviter_id.to_string())
                                )
                            })
                            .collect();
                        format!("{{\"invites\":[{}]}}", is.join(","))
                    }
                    "/group/accept" => match q("group") {
                        Some(gid) => match handle.accept_invite(gid.clone().into()).await {
                            Ok(()) => format!("{{\"ok\":true,\"accepte\":\"{gid}\"}}"),
                            Err(e) => format!("{{\"ok\":false,\"erreur\":\"{e}\"}}"),
                        },
                        None => "{\"ok\":false,\"erreur\":\"param 'group' manquant\"}".into(),
                    },
                    "/stop" => {
                        // Flush avant l'exit brutal : garantit la persistance de
                        // l'appartenance groupe + last_seq (rattrapage R13).
                        handle.save_now().await;
                        handle.shutdown().await;
                        let resp = "HTTP/1.1 200 OK\r\nContent-Length: 20\r\n\r\n{\"ok\":true,\"stop\":1}";
                        let _ = stream.write_all(resp.as_bytes()).await;
                        std::process::exit(0);
                    }
                    _ => "{\"erreur\":\"route inconnue\"}".into(),
                };

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
#[command(group(clap::ArgGroup::new("relay_enabled").args(["self_relay", "embedded_relay"]).multiple(true)))]
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
    #[arg(long, requires = "relay_enabled")]
    embedded_relay_publish: bool,

    /// Republication interval in seconds for RelayReadyAnnounce (publisher option, requires --embedded-relay-publish).
    /// Default: relay_ttl / 2.
    #[arg(long, value_name = "SECS", requires = "embedded_relay_publish")]
    relay_publish_interval: Option<u64>,

    /// Bind address for the embedded relay (default: [::]:0 = dual-stack, all interfaces).
    /// Use 127.0.0.1:0 for localhost-only, or a specific IP:PORT.
    #[arg(long, value_name = "ADDR", requires = "relay_enabled")]
    embedded_relay_bind: Option<std::net::SocketAddr>,

    /// Advertised IP for the embedded relay URL (overrides auto-detection).
    /// Use when auto-detection picks the wrong interface (e.g. VPN, Docker).
    #[arg(long, value_name = "IP", requires = "relay_enabled")]
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

    /// Appareil/platform identifier for structured logs (e.g. "macos", "linux", "nas").
    /// Auto-detected if not specified.
    #[arg(long, value_name = "PLATFORM")]
    node_appareil: Option<String>,

    /// Path to persistent identity file (32-byte Ed25519 secret key).
    /// If absent, a fresh ephemeral identity is generated on each run.
    /// Default when omitted: no persistence (ephemeral).
    #[arg(long, value_name = "PATH")]
    key_path: Option<std::path::PathBuf>,

    /// UDP host:port for centralized log collection (e.g. "192.168.1.10:9999").
    /// Logs are sent as JSON lines over UDP in addition to stderr.
    #[arg(long, value_name = "HOST:PORT")]
    log_udp: Option<String>,

    /// HTTP port for local status page (e.g. 8080).
    /// Exposes a JSON endpoint showing node identity, phase, peers, roles, metrics.
    #[arg(long, value_name = "PORT")]
    status_port: Option<u16>,

    /// HTTP port for the CONTROL API (e.g. 9100). Permet de piloter le nœud à
    /// distance : POST /send?to=<id>&size=<n>, POST /group/create, /group/send,
    /// /stop ; GET /peers, /groups. Pour l'orchestration de tests automatisée.
    #[arg(long, value_name = "PORT")]
    control_port: Option<u16>,

    /// Répertoire de persistance (state.db SQLite : état groupes, last_seq,
    /// historique hub R13). Absent → tout est éphémère (pas de rattrapage
    /// offline durable après restart).
    #[arg(long, value_name = "DIR")]
    data_dir: Option<std::path::PathBuf>,

    /// Mode isolé : coupe la découverte n0 (Pkarr/DNS) ET mDNS LAN. Le nœud ne
    /// parle qu'aux pairs joignables via relais/bootstrap explicites. Pour des
    /// triangles de test reproductibles sans polluer le vrai parc.
    #[arg(long)]
    isolated: bool,

    /// Fixed local UDP socket address for the QUIC endpoint (e.g. "[::]:43925").
    /// Binds a stable port instead of an OS-assigned ephemeral one — required
    /// for durable inbound firewall rules. Takes precedence over --bind-port.
    #[arg(long, value_name = "ADDR")]
    bind_addr: Option<std::net::SocketAddr>,

    /// Fixed local UDP port for the QUIC endpoint, bound on `[::]:PORT`
    /// (fixe le socket IPv6 ; le socket IPv4 reste sur port éphémère).
    /// Shorthand for --bind-addr. Ignored if --bind-addr is set.
    #[arg(long, value_name = "PORT")]
    bind_port: Option<u16>,

    /// One-shot message-size ramp (bot mode): send increasingly large payloads
    /// to the given peer NodeId, logging success/failure per size, then keep
    /// running in normal bot mode. Requires --bot.
    #[arg(long, value_name = "NODE_ID", requires = "bot")]
    size_ramp: Option<String>,

    /// Comma-separated payload sizes in bytes for --size-ramp.
    /// Defaults to a ramp from 1 KB up to and past the 1 MiB default ceiling.
    #[arg(long, value_name = "CSV", requires = "size_ramp")]
    ramp_sizes: Option<String>,

    /// Mode campagne (bot) : toutes les 5 s, envoie à chaque pair connecté un
    /// message de la taille du palier courant. Le palier dérive de l'horloge
    /// murale (epoch/120s % paliers) — les apps Swift calculent la même phase,
    /// donc tout le monde change de palier ensemble, sans coordination.
    #[arg(long, requires = "bot")]
    campaign: bool,
}

fn default_ramp_sizes() -> Vec<usize> {
    vec![
        1_000, 10_000, 50_000, 100_000, 250_000, 500_000, 750_000, 1_000_000, 1_048_576,
        1_100_000, 1_500_000, 2_000_000,
    ]
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
    let mut node_config = TomNodeConfig::new();
    if let Some(path) = cli.key_path.clone() {
        node_config = node_config.identity_path(path);
    }
    if cli.isolated {
        // Triangle de test reproductible : aucune découverte externe (ni n0, ni
        // mDNS LAN) pour ne pas se mélanger au vrai parc. Connexions via relais
        // embarqué + bootstrap explicites uniquement.
        node_config = node_config.n0_discovery(false).local_discovery(false);
    }
    // Fixed UDP bind: explicit --bind-addr wins, else --bind-port on [::] (dual-stack).
    let effective_bind_addr = cli.bind_addr.or_else(|| {
        cli.bind_port.map(|port| {
            std::net::SocketAddr::from((std::net::Ipv6Addr::UNSPECIFIED, port))
        })
    });
    if let Some(addr) = effective_bind_addr {
        node_config = node_config.bind_addr(addr);
    }
    let node = TomNode::bind(node_config).await?;
    let local_id = node.id();

    // Build runtime config
    let mut config = RuntimeConfig {
        username: cli.username.clone(),
        enable_transport_relay_discovery: cli.relay_discovery,
        enable_embedded_relay: use_embedded_relay,
        enable_embedded_relay_publication: use_embedded_relay_publish,
        data_dir: cli.data_dir.clone(),
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

    // Per-peer path tracking: node_id → (kind, rtt_ms, addr)
    let paths_by_peer = Arc::new(Mutex::new(PathsByPeerMap::new()));

    // Start status HTTP server if requested
    if let Some(port) = cli.status_port {
        spawn_status_server(port, handle.clone(), cli.node_label.clone(), paths_by_peer.clone());
    }

    // Inbox partagé pour l'observabilité des messages reçus (endpoint /inbox).
    let inbox: Inbox = Arc::new(Mutex::new(VecDeque::new()));

    if let Some(port) = cli.control_port {
        spawn_control_server(port, handle.clone(), inbox.clone());
    }

    if cli.bot {
        let udp = cli.log_udp.as_deref().and_then(UdpLogger::new);
        let appareil = cli.node_appareil.unwrap_or_else(|| {
            if cfg!(target_os = "macos") { "macos".into() }
            else if cfg!(target_os = "linux") { "linux".into() }
            else { "unknown".into() }
        });
        let ctx = Arc::new(BotContext {
            node_label: cli.node_label.clone(),
            node_id: local_id.to_string(),
            appareil,
            udp,
            handle: handle.clone(),
        });
        ctx.log_event("demarrage", &format!("noeud={} mode=bot", cli.node_label));

        if let Some(ref target_str) = cli.size_ramp {
            match target_str.parse::<NodeId>() {
                Ok(target) => {
                    let ramp_sizes = cli
                        .ramp_sizes
                        .as_deref()
                        .map(|s| {
                            s.split(',')
                                .filter_map(|p| p.trim().parse::<usize>().ok())
                                .collect::<Vec<_>>()
                        })
                        .filter(|v: &Vec<usize>| !v.is_empty())
                        .unwrap_or_else(default_ramp_sizes);
                    let ramp_ctx = ctx.clone();
                    let ramp_handle = handle.clone();
                    tokio::spawn(async move {
                        run_size_ramp(ramp_ctx, ramp_handle, target, ramp_sizes).await;
                    });
                }
                Err(e) => ctx.log_event("size_ramp_erreur", &format!("node_id invalide: {}", e)),
            }
        }

        if cli.campaign {
            let camp_ctx = ctx.clone();
            let camp_handle = handle.clone();
            tokio::spawn(async move {
                run_campaign(camp_ctx, camp_handle).await;
            });
        }

        return run_bot(ctx, handle, messages, events, cli.bot_ping, inbox, paths_by_peer).await;
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
                    KeyCode::Enter if !app.input.is_empty() => {
                        let text = app.input.drain(..).collect::<String>();
                        handle_input(&mut app, &text, &handle).await;
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
            // Track path changes per peer (same as bot mode, for status server consistency)
            track_path_event(&paths_by_peer, &evt);
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
        ProtocolEvent::PeerDiscovered { node_id, username, source, .. } => {
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
    inbox: Inbox,
    paths_by_peer: Arc<Mutex<PathsByPeerMap>>,
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

                // Capture pour l'endpoint /inbox (observabilité tests R13).
                {
                    let mut guard = inbox.lock().unwrap();
                    guard.push_back(InboxEntry {
                        ts: msg.timestamp,
                        from: msg.from.to_string(),
                        text: text.to_string(),
                    });
                    while guard.len() > INBOX_MAX {
                        guard.pop_front();
                    }
                }

                // Affichage compact : jamais le payload brut complet (un CAMP
                // de 250 Ko ferait exploser le datagramme UDP de log).
                let affichage = if let Some(rest) = text.strip_prefix("CAMP:") {
                    let mut it = rest.splitn(3, ':');
                    let taille = it.next().unwrap_or("?");
                    let seq = it.next().unwrap_or("?");
                    format!("camp taille={taille} seq={seq}")
                } else {
                    text.chars().take(80).collect::<String>()
                };
                ctx.log_event("message_recu", &format!(
                    "de={} sig={} contenu={}",
                    short_node_id(&msg.from),
                    if msg.signature_valid { "ok" } else { "bad" },
                    affichage
                ));

                // Reply with fixed-size response — jamais à un écho (chaîne coupée net)
                if !text.starts_with("recu 5/5") {
                    let reply = format!("recu 5/5 (msg #{})", count);
                    match handle.send_message(msg.from, reply.as_bytes().to_vec()).await {
                        Ok(()) => ctx.log_event("reponse_envoyee", &format!("a={}", short_node_id(&msg.from))),
                        Err(e) => ctx.log_event("erreur_envoi", &e.to_string()),
                    }
                }
            }
            evt_opt = events.recv() => {
                let Some(evt) = evt_opt else { break; };
                // Track path changes per peer (both bot and TUI modes)
                track_path_event(&paths_by_peer, &evt);
                if let ProtocolEvent::PathChanged { event } = &evt {
                    // Log path changes to collecteur
                    ctx.log_event("path_change", &format!(
                        "pair={} kind={} rtt_ms={} addr={}",
                        short_node_id(&event.remote),
                        event.kind,
                        event.rtt.as_millis(),
                        event.addr
                    ));
                }
                // Capture des messages de groupe (canal events, pas DeliveredMessage)
                // pour l'endpoint /inbox — indispensable à la validation R13.
                if let ProtocolEvent::GroupMessageReceived { message } = &evt {
                    let body = if message.text.is_empty() {
                        format!("[chiffré {}o]", message.ciphertext.len())
                    } else {
                        message.text.clone()
                    };
                    ctx.log_event("groupe_message_recu", &format!(
                        "grp={} de={} seq={} contenu={}",
                        message.group_id,
                        short_node_id(&message.sender_id),
                        message.seq,
                        body.chars().take(60).collect::<String>()
                    ));
                    let mut guard = inbox.lock().unwrap();
                    guard.push_back(InboxEntry {
                        ts: message.sent_at,
                        from: format!("{} grp={} seq={}", message.sender_id, message.group_id, message.seq),
                        text: body,
                    });
                    while guard.len() > INBOX_MAX {
                        guard.pop_front();
                    }
                }
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

/// One-shot escalating payload-size send to `target`, logging pass/fail per size.
/// Stops ramping on the first failure (send-side rejection or transport error)
/// so the ceiling is visible directly in the log stream.
/// Paliers du mode campagne — MÊMES valeurs que `TomNodeService.campaignSizes`
/// (apps Swift) : phase dérivée de l'horloge murale, changement synchronisé.
const CAMPAIGN_SIZES: [usize; 6] = [1_000, 10_000, 50_000, 100_000, 150_000, 250_000];
const CAMPAIGN_PHASE_SECS: u64 = 120;

async fn run_campaign(ctx: Arc<BotContext>, handle: RuntimeHandle) {
    ctx.log_event(
        "campagne_debut",
        &format!("paliers={CAMPAIGN_SIZES:?} phase={CAMPAIGN_PHASE_SECS}s"),
    );
    let mut seq: u64 = 0;
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        let epoch = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let size = CAMPAIGN_SIZES[((epoch / CAMPAIGN_PHASE_SECS) as usize) % CAMPAIGN_SIZES.len()];
        for peer in handle.connected_peers().await {
            seq += 1;
            let header = format!("CAMP:{size}:{seq}:");
            let mut payload = header.into_bytes();
            payload.resize(size.max(payload.len()), b'A');
            match handle.send_message(peer, payload).await {
                Ok(()) => ctx.log_event(
                    "camp_envoi",
                    &format!("taille={size} seq={seq} vers={}", short_node_id(&peer)),
                ),
                Err(e) => ctx.log_event(
                    "camp_echec",
                    &format!("taille={size} seq={seq} vers={} erreur={e}", short_node_id(&peer)),
                ),
            }
        }
    }
}

async fn run_size_ramp(ctx: Arc<BotContext>, handle: RuntimeHandle, target: NodeId, sizes: Vec<usize>) {
    ctx.log_event(
        "size_ramp_debut",
        &format!("cible={} paliers={:?}", short_node_id(&target), sizes),
    );

    // Ensure the target is a known peer before ramping — a cold send right at
    // startup can race gossip/DHT discovery and fail on the very first size.
    handle.add_peer(target).await;
    tokio::time::sleep(Duration::from_secs(3)).await;

    for size in sizes {
        let payload = vec![b'A'; size];
        let start = Instant::now();
        match handle.send_message(target, payload).await {
            Ok(()) => {
                ctx.log_event(
                    "size_ramp_ok",
                    &format!("taille={} ms={}", size, start.elapsed().as_millis()),
                );
            }
            Err(e) => {
                ctx.log_event("size_ramp_echec", &format!("taille={} erreur={}", size, e));
                break;
            }
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }

    ctx.log_event("size_ramp_fin", "termine");
}

fn handle_bot_event(ctx: &BotContext, event: &ProtocolEvent) {
    match event {
        ProtocolEvent::PeerDiscovered { node_id, username, source, .. } => {
            ctx.log_event_sourced(
                "pair_trouve",
                &format!("{} \"{}\"", short_node_id(node_id), username),
                Some(discovery_source_str(source)),
            );
        }
        ProtocolEvent::PeerStale { node_id } => {
            ctx.log_event("pair_stale", &short_node_id(node_id));
        }
        ProtocolEvent::PeerOffline { node_id } => {
            ctx.log_event("pair_offline", &short_node_id(node_id));
        }
        ProtocolEvent::PeerOnline { node_id } => {
            ctx.log_event("pair_online", &short_node_id(node_id));
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
        ProtocolEvent::Error { description } => {
            ctx.log_event("erreur_protocole", description);
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

fn discovery_source_str(source: &tom_protocol::DiscoverySource) -> &'static str {
    match source {
        tom_protocol::DiscoverySource::Mdns        => "mdns",
        tom_protocol::DiscoverySource::PeerPresent => "peer_present",
        tom_protocol::DiscoverySource::Dht         => "dht",
        tom_protocol::DiscoverySource::Gossip      => "gossip",
        tom_protocol::DiscoverySource::Announce    => "announce",
        tom_protocol::DiscoverySource::Direct      => "direct",
    }
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
            app_build: 0,
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
