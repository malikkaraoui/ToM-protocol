//! FFI wrapper for tom-protocol — exposes C API for Swift/tvOS
//!
//! Architecture:
//! - Swift calls tom_node_create() with JSON config → returns opaque TomNodeHandle
//! - Swift calls tom_node_start() → spawns ProtocolRuntime
//! - Swift polls tom_node_receive_messages() → batch JSON of incoming messages
//! - Swift calls tom_node_send_message() / tom_node_create_group() → commands
//! - Swift calls tom_node_stop() → shutdown + cleanup
//!
//! All async operations are managed by an internal tokio runtime.

use std::collections::{HashMap, VecDeque};
use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::sync::Arc;
use tokio::runtime::Runtime;
use tokio::sync::Mutex;
use tracing_subscriber::EnvFilter;
use tracing_subscriber::prelude::*;

use tom_protocol::{DeliveredMessage, ProtocolEvent, ProtocolRuntime, RuntimeChannels, RuntimeConfig, RuntimeHandle};
use tom_transport::TomNodeConfig;

mod types;
use types::{DeliveredMessageFFI, DiscoveredPeerFFI, GroupConfigFFI, NodeConfigFFI, NodeStatusFFI, PeerAddrFFI, PathInfoFFI, PresenceMetricsFFI, PresenceStatsFFI, ProtocolEventFFI, RuntimeConfigFFI};

/// Type alias for per-peer path info: node_id → PathInfoFFI
type PathsByPeerMap = HashMap<String, PathInfoFFI>;

/// Hard cap for discovered peer cache exposed over FFI.
/// Prevents unbounded growth on long-running tvOS sessions.
const MAX_DISCOVERED_PEERS: usize = 2048;

/// Hard cap for per-peer path tracking cache.
/// Prevents unbounded growth in the paths_by_peer HashMap.
const MAX_PATH_ENTRIES: usize = 2048;

/// Opaque handle to the TOM protocol node (passed to/from Swift as void*)
pub struct TomNodeHandle {
    runtime: Runtime,
    handle: Arc<Mutex<Option<RuntimeHandle>>>,
    /// Buffered messages from runtime (polled by Swift)
    message_queue: Arc<Mutex<VecDeque<DeliveredMessage>>>,
    /// Buffered events from runtime (for status/debug) — excludes GroupMessageReceived
    event_queue: Arc<Mutex<VecDeque<ProtocolEvent>>>,
    /// Buffered group message events (polled separately by Swift) — isolated queue
    group_message_queue: Arc<Mutex<VecDeque<ProtocolEvent>>>,
    /// Peers discovered via gossip/DHT (cached from PeerDiscovered events)
    discovered_peers: Arc<Mutex<HashMap<String, DiscoveredPeerFFI>>>,
    /// Node ID (cached after bind)
    node_id: Arc<Mutex<Option<String>>>,
    /// Last error message (retrievable by Swift after a -1 return)
    last_error: Arc<std::sync::Mutex<Option<String>>>,
    /// Last known paths by peer: node_id → (kind, rtt_ms, addr) (updated from PathChanged events)
    last_paths: Arc<Mutex<PathsByPeerMap>>,
    /// Local node role: "Peer" or "Relay" (updated from LocalRoleChanged events)
    local_role: Arc<Mutex<String>>,
    /// Relay URL passed at start time (stored for status reporting).
    configured_relay_url: Arc<std::sync::Mutex<Option<String>>>,
    /// L1-001 presence: (accepted_total, last_attester, last_latency_ms) —
    /// updated from PresenceAttestationReceived events, polled by Swift.
    presence_stats: Arc<Mutex<(u64, String, u64)>>,
    /// Application build number (non-authoritative hint, stored at start time).
    app_build: Arc<std::sync::Mutex<u32>>,
}

/// Initialize tracing (logs) for the node
fn init_tracing() {
    let _ = tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(EnvFilter::from_default_env())
        .try_init();
}

/// Budget des appels FFI courants (polls/statuts/envois). Un runtime gelé
/// rend None au lieu de gonfler le budget de l'actor Swift.
const FFI_CALL_BUDGET: std::time::Duration = std::time::Duration::from_secs(2);

/// Exécute `fut` sur le runtime avec un budget tenu par le THREAD APPELANT
/// (recv_timeout, primitive OS — immune à un timer tokio mort) : un runtime
/// gelé (deadlock transport) rend None au lieu de geler l'appel C — l'actor
/// Swift ne peut plus être empoisonné par un poll.
fn bounded_block_on<T, F>(runtime: &tokio::runtime::Runtime, budget: std::time::Duration, fut: F) -> Option<T>
where
    T: Send + 'static,
    F: std::future::Future<Output = T> + Send + 'static,
{
    let (tx, rx) = std::sync::mpsc::channel();
    runtime.spawn(async move {
        let _ = tx.send(fut.await);
    });
    rx.recv_timeout(budget).ok()
}

/// Drain TRANSACTIONNEL d'une file sous budget appelant.
///
/// Contrairement à [`bounded_block_on`], la file n'est vidée QUE si le
/// résultat est effectivement récupéré par l'appelant : si le budget expire
/// (runtime gelé) et que la tâche s'exécute plus tard, elle ne draine pas —
/// sinon un dégel jetterait silencieusement tout le batch (messages perdus,
/// exactement la classe de bug qu'on combat).
fn bounded_drain_json<T, F>(
    runtime: &tokio::runtime::Runtime,
    budget: std::time::Duration,
    queue: Arc<Mutex<VecDeque<T>>>,
    serialize: F,
) -> String
where
    T: Clone + Send + 'static,
    F: FnOnce(Vec<T>) -> String + Send + 'static,
{
    // Commit en DEUX PHASES : la tâche n'efface la file qu'après un ACK
    // explicite de l'appelant. Ni perte (le clear n'arrive jamais si le JSON
    // n'a pas été récupéré) ni doublon (le clear arrive toujours s'il l'a
    // été) — quelle que soit l'imbrication timeout/send.
    let (tx, rx) = std::sync::mpsc::channel::<String>();
    let (ack_tx, ack_rx) = std::sync::mpsc::channel::<()>();
    let alive = Arc::new(std::sync::atomic::AtomicBool::new(true));
    let alive_task = alive.clone();
    runtime.spawn(async move {
        let mut q = queue.lock().await;
        if !alive_task.load(std::sync::atomic::Ordering::Acquire) {
            // L'appelant a expiré avant même qu'on obtienne la file :
            // ne rien drainer, le prochain poll reprendra tout.
            return;
        }
        let items: Vec<T> = q.iter().cloned().collect();
        let json = serialize(items);
        if tx.send(json).is_err() {
            return;
        }
        // Attente bornée de l'ACK (µs sur runtime sain — l'appelant est déjà
        // dans recv_timeout ; l'appelant disparu → pas de clear, batch gardé).
        if ack_rx.recv_timeout(std::time::Duration::from_millis(100)).is_ok() {
            q.clear();
        }
    });
    match rx.recv_timeout(budget) {
        Ok(json) => {
            let _ = ack_tx.send(());
            json
        }
        Err(_) => {
            alive.store(false, std::sync::atomic::Ordering::Release);
            // La tâche a pu envoyer PILE pendant l'expiration : récupérer et
            // ACKer pour que le drain soit commis — jamais jeté, jamais doublé.
            match rx.try_recv() {
                Ok(json) => {
                    let _ = ack_tx.send(());
                    json
                }
                Err(_) => {
                    tracing::warn!("drain FFI expiré (runtime occupé/gelé) — batch conservé");
                    "[]".to_string()
                }
            }
        }
    }
}

fn normalized_non_empty(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

fn parse_gossip_bootstrap_peers(peers: &[String]) -> Vec<tom_protocol::types::NodeId> {
    peers
        .iter()
        .filter_map(|s| normalized_non_empty(Some(s.as_str())))
        .filter_map(|s| s.parse().ok())
        .collect()
}

/// Safely read a C string argument across the FFI boundary.
///
/// Returns `None` when the pointer is null or the bytes are not valid UTF-8,
/// instead of triggering undefined behaviour. `CStr::from_ptr` on a null
/// pointer is UB, and the caller (Swift) can legitimately bridge a `nil`
/// `String` to a null `char*`; every C-string argument must go through here.
///
/// # Safety
/// If non-null, `ptr` must point to a valid null-terminated C string that
/// stays alive for the duration of the returned borrow.
unsafe fn cstr_opt<'a>(ptr: *const c_char) -> Option<&'a str> {
    if ptr.is_null() {
        return None;
    }
    match unsafe { CStr::from_ptr(ptr) }.to_str() {
        Ok(s) => Some(s),
        Err(e) => {
            tracing::error!("FFI received non-UTF-8 C string: {}", e);
            None
        }
    }
}

/// Lock a std mutex without panicking across the FFI boundary on poison.
///
/// A panic unwinding into Swift is undefined behaviour, so a poisoned lock
/// (a thread panicked while holding it) must be recovered rather than
/// re-panicked via `.unwrap()`.
fn lock_recover<T>(m: &std::sync::Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Create a TOM protocol node (but don't start it yet)
///
/// # Arguments
/// * `config_json` - JSON string with NodeConfig fields (username, relay_url, etc.)
///
/// # Returns
/// * Opaque pointer to TomNodeHandle on success
/// * NULL on failure (check logs for details)
///
/// # Safety
/// * Caller must call `tom_node_free()` to free resources
/// * `config_json` must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_create(config_json: *const c_char) -> *mut TomNodeHandle {
    init_tracing();

    // Parse JSON config (null-safe + UTF-8 safe)
    let config_str = match unsafe { cstr_opt(config_json) } {
        Some(s) => s,
        None => {
            tracing::error!("tom_node_create: null or non-UTF-8 config_json");
            return std::ptr::null_mut();
        }
    };

    let _config: NodeConfigFFI = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Invalid JSON config: {}", e);
            return std::ptr::null_mut();
        }
    };

    // Create tokio runtime
    let runtime = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            tracing::error!("Failed to create tokio runtime: {}", e);
            return std::ptr::null_mut();
        }
    };

    tracing::info!("Created TomNodeHandle (not started yet)");

    // Store config in handle for later start
    Box::into_raw(Box::new(TomNodeHandle {
        runtime,
        handle: Arc::new(Mutex::new(None)),
        message_queue: Arc::new(Mutex::new(VecDeque::new())),
        event_queue: Arc::new(Mutex::new(VecDeque::new())),
        group_message_queue: Arc::new(Mutex::new(VecDeque::new())),
        discovered_peers: Arc::new(Mutex::new(HashMap::new())),
        node_id: Arc::new(Mutex::new(None)),
        last_error: Arc::new(std::sync::Mutex::new(None)),
        last_paths: Arc::new(Mutex::new(HashMap::new())),
        local_role: Arc::new(Mutex::new("Peer".to_string())),
        configured_relay_url: Arc::new(std::sync::Mutex::new(None)),
        presence_stats: Arc::new(Mutex::new((0, String::new(), 0))),
        app_build: Arc::new(std::sync::Mutex::new(0)),
    }))
}

/// Start the protocol runtime
///
/// # Arguments
/// * `handle` - Opaque handle returned by `tom_node_create()`
/// * `runtime_config_json` - JSON string with RuntimeConfig fields (encryption, username, etc.)
///
/// # Returns
/// * 0 on success
/// * -1 on failure (details via `tom_node_last_error`)
/// * -2 when the start budget expired (`start_timeout_secs`, default 20s):
///   network unavailable/degraded — the handle stays reusable, retry later
///
/// # Safety
/// * `handle` must be a valid pointer returned by `tom_node_create()`
/// * `runtime_config_json` must be a valid null-terminated C string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_start(
    handle: *mut TomNodeHandle,
    runtime_config_json: *const c_char,
) -> i32 {
    if handle.is_null() {
        tracing::error!("tom_node_start: NULL handle");
        return -1;
    }

    let handle_ref = unsafe { &*handle };

    // Parse runtime config (null-safe + UTF-8 safe)
    let config_str = match unsafe { cstr_opt(runtime_config_json) } {
        Some(s) => s,
        None => {
            tracing::error!("tom_node_start: null or non-UTF-8 runtime_config_json");
            return -1;
        }
    };

    let runtime_config: RuntimeConfigFFI = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Invalid JSON runtime config: {}", e);
            return -1;
        }
    };

    // Build transport config
    let mut transport_config = TomNodeConfig::new();

    if let Some(relay_url) = normalized_non_empty(runtime_config.relay_url.as_deref()) {
        if let Ok(url) = relay_url.parse::<tom_connect::RelayUrl>() {
            *lock_recover(&handle_ref.configured_relay_url) = Some(relay_url.clone());
            transport_config = transport_config.relay_url(url);
        }
    }

    if let Some(identity_path) = &runtime_config.identity_path {
        transport_config = transport_config.identity_path(identity_path.into());
    }

    if let Some(n0_discovery) = runtime_config.n0_discovery {
        transport_config = transport_config.n0_discovery(n0_discovery);
    }

    if let Some(local_discovery) = runtime_config.local_discovery {
        transport_config = transport_config.local_discovery(local_discovery);
    }

    if let Some(discovery_url) = normalized_non_empty(runtime_config.relay_discovery_url.as_deref()) {
        transport_config = transport_config.relay_discovery_url(discovery_url);
    }

    // Parse gossip bootstrap peers
    let gossip_peers = parse_gossip_bootstrap_peers(&runtime_config.gossip_bootstrap_peers);

    // Build protocol config
    let protocol_config = RuntimeConfig {
        username: runtime_config.username.clone(),
        encryption: runtime_config.encryption.unwrap_or(true),
        enable_dht: runtime_config.enable_dht.unwrap_or(true),
        data_dir: runtime_config.data_dir.map(|p| p.into()),
        gossip_bootstrap_peers: gossip_peers,
        // Full-node mode: every node is simultaneously a relay server
        enable_embedded_relay: runtime_config.enable_embedded_relay.unwrap_or(true),
        enable_embedded_relay_publication: runtime_config.enable_embedded_relay_publication.unwrap_or(true),
        enable_transport_relay_discovery: runtime_config.enable_transport_relay_discovery.unwrap_or(true),
        presence_contribution_min: runtime_config
            .presence_contribution_min
            .unwrap_or(tom_protocol::presence::RELAY_CONTRIBUTION_MIN),
        presence_probe_interval: runtime_config
            .presence_probe_interval_secs
            .filter(|&s| s > 0)
            .map(|s| std::time::Duration::from_secs(s as u64)),
        app_build: runtime_config.app_build.unwrap_or(0),
        ..Default::default()
    };

    let handle_clone = handle_ref.handle.clone();
    let msg_queue = handle_ref.message_queue.clone();
    let event_queue = handle_ref.event_queue.clone();
    let group_message_queue = handle_ref.group_message_queue.clone();
    let node_id_arc = handle_ref.node_id.clone();
    let last_error_arc = handle_ref.last_error.clone();
    let app_build_value = runtime_config.app_build.unwrap_or(0);

    // Clear previous error
    *lock_recover(&last_error_arc) = None;

    // Store app build for status reporting
    *lock_recover(&handle_ref.app_build) = app_build_value;

    // Budget de démarrage : un appel C gelé n'est pas annulable côté Swift et
    // empoisonne l'actor appelant (stop/forceReset s'empilent derrière pour
    // toujours) pendant qu'un éventuel nœud précédent survit en zombie et ACK
    // des messages que l'UI ne verra jamais. Le start DOIT donc rendre la main
    // dans un délai borné, quoi qu'il arrive au réseau.
    // clamp haut 300 s : garde-fou contre une valeur aberrante (confusion
    // ms/s côté appelant) qui recréerait de fait un start non borné.
    let start_timeout = std::time::Duration::from_secs(
        runtime_config.start_timeout_secs.unwrap_or(20).clamp(1, 300),
    );

    // Clones sortis du futur : le travail part en tâche 'static sur le runtime.
    let discovered_peers_clone = handle_ref.discovered_peers.clone();
    let last_paths_clone = handle_ref.last_paths.clone();
    let local_role_clone = handle_ref.local_role.clone();
    let presence_stats_clone = handle_ref.presence_stats.clone();

    // Gate anti-zombie : un seul gagnant. COMMITTED = le travail publie le
    // runtime ; ABANDONED = l'appelant a expiré → un nœud qui aboutirait
    // APRÈS l'expiration est démantelé au lieu d'être publié (jamais de
    // runtime orphelin qui ACK sans app derrière).
    const GATE_PENDING: u8 = 0;
    const GATE_COMMITTED: u8 = 1;
    const GATE_ABANDONED: u8 = 2;
    let gate = Arc::new(std::sync::atomic::AtomicU8::new(GATE_PENDING));
    let gate_work = gate.clone();

    let work = async move {
        tracing::info!("Binding TomNode (budget {}s)...", start_timeout.as_secs());
        let start_t0 = std::time::Instant::now();

        // Bind transport — sur une tâche séparée : même si une étape interne
        // bloque un worker en synchrone, le timeout ci-dessous expire quand même.
        let mut bind_task = tokio::spawn(tom_transport::TomNode::bind(transport_config));

        let node = match tokio::time::timeout(start_timeout, &mut bind_task).await {
            Ok(Ok(Ok(n))) => {
                let id = n.id().to_string();
                tracing::info!(
                    "TomNode bound successfully in {}ms: {}",
                    start_t0.elapsed().as_millis(),
                    id
                );
                *node_id_arc.lock().await = Some(id);
                n
            }
            Ok(Ok(Err(e))) => {
                let err_msg = format!("Failed to bind TomNode: {}", e);
                tracing::error!("{}", err_msg);
                *lock_recover(&last_error_arc) = Some(err_msg);
                return -1i32;
            }
            Ok(Err(join_err)) => {
                let err_msg = format!("TomNode bind task failed: {}", join_err);
                tracing::error!("{}", err_msg);
                *lock_recover(&last_error_arc) = Some(err_msg);
                return -1i32;
            }
            Err(_elapsed) => {
                // Start expiré. RIEN ne doit survivre à ce chemin d'erreur :
                // on annule le bind, et si la course abort/complétion laisse
                // malgré tout un nœud entièrement bindé, il est démantelé
                // immédiatement — sinon on recrée exactement le zombie qu'on
                // combat (endpoint vivant qui ACK sans app derrière).
                bind_task.abort();
                tokio::spawn(async move {
                    if let Ok(Ok(node)) = bind_task.await {
                        tracing::warn!(
                            "bind terminé après l'expiration du start — démantèlement immédiat"
                        );
                        let _ = node.shutdown().await;
                    }
                });
                let err_msg = format!(
                    "démarrage expiré après {}s — réseau indisponible ou dégradé, réessayer",
                    start_timeout.as_secs()
                );
                tracing::error!("{}", err_msg);
                *lock_recover(&last_error_arc) = Some(err_msg);
                // -2 = contrat « expiré » : l'appelant peut réessayer sur le
                // même handle avec backoff, sans matcher le texte d'erreur.
                return -2i32;
            }
        };

        // Point de non-retour : si l'appelant a déjà abandonné (budget OS
        // expiré pendant un gel du runtime), ce nœud tardif ne doit PAS être
        // publié — on le démantèle immédiatement.
        if gate_work
            .compare_exchange(
                GATE_PENDING,
                GATE_COMMITTED,
                std::sync::atomic::Ordering::SeqCst,
                std::sync::atomic::Ordering::SeqCst,
            )
            .is_err()
        {
            tracing::warn!("start abandonné par l'appelant — démantèlement du nœud tardif");
            let _ = node.shutdown().await;
            return -2i32;
        }

        // Spawn protocol runtime — chemin rapide et LOCAL par contrat : plus
        // aucune étape réseau synchrone ici (l'init DHT/DNS est détachée,
        // cf. tom_dht::SharedDht). Le log de durée trahirait une régression.
        let spawn_t0 = std::time::Instant::now();
        let channels: RuntimeChannels = ProtocolRuntime::spawn(node, protocol_config);

        tracing::info!(
            "ProtocolRuntime spawned successfully in {}ms (start total {}ms)",
            spawn_t0.elapsed().as_millis(),
            start_t0.elapsed().as_millis()
        );
        *handle_clone.lock().await = Some(channels.handle.clone());

        // Background task: drain messages + events into queues
        let msg_queue_clone = msg_queue.clone();
        let event_queue_clone = event_queue.clone();
        let group_message_queue_clone = group_message_queue.clone();
        let mut messages_rx = channels.messages;
        let mut events_rx = channels.events;
        let mut status_rx = channels.status_changes;
        let status_event_queue = event_queue_clone.clone();

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(msg) = messages_rx.recv() => {
                        let mut q = msg_queue_clone.lock().await;
                        q.push_back(msg);
                        // Keep queue bounded — drop oldest if too large
                        while q.len() > 1000 {
                            q.pop_front();
                        }
                    }
                    // Statut de livraison par message (Pending→Sent→Relayed→
                    // Delivered/Failed). Réinjecté dans la même file d'events que
                    // le reste, en ProtocolEvent, pour l'UI Swift (statut visible).
                    Some(change) = status_rx.recv() => {
                        // Ignore les transitions NO-OP (previous == current) : le
                        // tracker émet un seed Pending→Pending à l'enregistrement,
                        // immédiatement suivi de Pending→Sent. Sans ce filtre, l'UI
                        // recevait DEUX events « en cours » par message (Pending ET
                        // Sent mappent tous deux sur « en cours ») → doublons
                        // observés dans le Live Log. On ne surface qu'un vrai
                        // changement d'état.
                        if change.previous != change.current {
                            let mut q = status_event_queue.lock().await;
                            q.push_back(ProtocolEvent::MessageStatusChanged {
                                message_id: change.message_id,
                                to: change.to,
                                status: change.current,
                            });
                            while q.len() > 1000 {
                                q.pop_front();
                            }
                        }
                    }
                    Some(event) = events_rx.recv() => {
                        // Track path upgrades (RELAY ↔ DIRECT) per peer
                        if let ProtocolEvent::PathChanged { ref event } = event {
                            let mut lp = last_paths_clone.lock().await;
                            let node_id = event.remote.to_string();
                            // Bounded insert: ignore if at capacity and key is new
                            if lp.contains_key(&node_id) || lp.len() < MAX_PATH_ENTRIES {
                                lp.insert(node_id, PathInfoFFI {
                                    kind: format!("{}", event.kind),
                                    rtt_ms: event.rtt.as_millis() as u64,
                                    addr: event.addr.clone(),
                                });
                            }
                        }
                        // Purge path entries on peer offline/stale
                        if let ProtocolEvent::PeerOffline { ref node_id } | ProtocolEvent::PeerStale { ref node_id } = event {
                            let mut lp = last_paths_clone.lock().await;
                            lp.remove(&node_id.to_string());
                        }
                        // Track local role changes
                        if let ProtocolEvent::LocalRoleChanged { ref new_role } = event {
                            let mut lr = local_role_clone.lock().await;
                            *lr = format!("{:?}", new_role);
                        }
                        // L1-001: track accepted presence attestations
                        if let ProtocolEvent::PresenceAttestationReceived {
                            ref attester_id,
                            ref latency_ms,
                            ..
                        } = event
                        {
                            tracing::info!(
                                "Presence attestation from {} in {}ms",
                                attester_id,
                                latency_ms
                            );
                            let mut ps = presence_stats_clone.lock().await;
                            ps.0 += 1;
                            ps.1 = attester_id.to_string();
                            ps.2 = *latency_ms;
                        }
                        // Cache discovered peers from gossip/DHT
                        if let ProtocolEvent::PeerDiscovered { ref node_id, ref username, ref source, ref app_build } = event {
                            let incoming = DiscoveredPeerFFI {
                                node_id: node_id.to_string(),
                                username: username.clone(),
                                app_build: *app_build,
                                source: format!("{:?}", source),
                                discovered_at: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64,
                            };
                            tracing::info!("Peer discovered: {} ({})", username, node_id);

                            let mut peers = discovered_peers_clone.lock().await;

                            // Merge with existing entry if present. Preserve a non-empty username
                            // when incoming value is empty (best-effort against sparse events).
                            let merged = if let Some(existing) = peers.get(&incoming.node_id) {
                                DiscoveredPeerFFI {
                                    node_id: incoming.node_id.clone(),
                                    username: if incoming.username.is_empty() {
                                        existing.username.clone()
                                    } else {
                                        incoming.username.clone()
                                    },
                                    // Anti-clobber: an event without a build (0) must
                                    // not erase a build already learned for this peer.
                                    app_build: if incoming.app_build == 0 {
                                        existing.app_build
                                    } else {
                                        incoming.app_build
                                    },
                                    source: incoming.source.clone(),
                                    discovered_at: incoming.discovered_at,
                                }
                            } else {
                                incoming
                            };

                            peers.insert(node_id.to_string(), merged);

                            // Enforce bounded cache size (evict oldest discovered entries first).
                            while peers.len() > MAX_DISCOVERED_PEERS {
                                if let Some(oldest_key) = peers
                                    .iter()
                                    .min_by_key(|(_, p)| p.discovered_at)
                                    .map(|(k, _)| k.clone())
                                {
                                    peers.remove(&oldest_key);
                                } else {
                                    break;
                                }
                            }
                        }

                        // Route GroupMessageReceived to dedicated queue, all others to event_queue
                        if matches!(event, ProtocolEvent::GroupMessageReceived { .. }) {
                            let mut gmq = group_message_queue_clone.lock().await;
                            gmq.push_back(event);
                            while gmq.len() > 500 {
                                gmq.pop_front();
                            }
                        } else {
                            let mut eq = event_queue_clone.lock().await;
                            eq.push_back(event);
                            while eq.len() > 500 {
                                eq.pop_front();
                            }
                        }
                    }
                    else => break,
                }
            }
            tracing::warn!("Message/event pump stopped");
        });

        0i32
    };

    // Le résultat repasse par une primitive OS (mpsc std) et le budget est
    // tenu par le THREAD APPELANT : un runtime tokio entièrement gelé
    // (deadlock transport — stall 5-15 min mesuré ; os.log iPad 17/07 : le
    // timeout tokio n'a jamais tiré en 5 min 21 s) ne peut plus geler
    // l'appel C ni empoisonner l'actor Swift. Marge +5 s au-dessus du budget
    // fin : le chemin nominal (timeout tokio interne, erreurs précises)
    // garde la priorité, le filet OS ne claque que sur runtime mort.
    let (done_tx, done_rx) = std::sync::mpsc::channel::<i32>();
    handle_ref.runtime.spawn(async move {
        let _ = done_tx.send(work.await);
    });

    match done_rx.recv_timeout(start_timeout + std::time::Duration::from_secs(5)) {
        Ok(rc) => rc,
        Err(_) => {
            if gate
                .compare_exchange(
                    GATE_PENDING,
                    GATE_ABANDONED,
                    std::sync::atomic::Ordering::SeqCst,
                    std::sync::atomic::Ordering::SeqCst,
                )
                .is_ok()
            {
                let msg = format!(
                    "runtime transport gelé — démarrage abandonné après {}s ; libérer ce nœud et en recréer un",
                    start_timeout.as_secs() + 5
                );
                tracing::error!("{}", msg);
                *lock_recover(&handle_ref.last_error) = Some(msg);
                -2
            } else {
                // Le travail a committé pile à l'expiration : son rc arrive.
                done_rx
                    .recv_timeout(std::time::Duration::from_secs(2))
                    .unwrap_or(-2)
            }
        }
    }
}

/// Stop the node and free all resources
///
/// # Safety
/// * `handle` must be a valid pointer returned by `tom_node_create()`
/// * After calling this, `handle` is invalid and must not be used
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_stop(handle: *mut TomNodeHandle) {
    if handle.is_null() {
        tracing::warn!("tom_node_stop: NULL handle");
        return;
    }

    detached_teardown(unsafe { Box::from_raw(handle) }, true);
}

/// Tear down a node handle on a DETACHED thread so the caller never blocks.
///
/// Dropping the Runtime / joining the DHT (mainline) background thread can block
/// for seconds — and a synchronous `Drop` can't be bounded by `shutdown_timeout`
/// — so we move the whole teardown off the calling thread.
///
/// OWNERSHIP CONTRACT: this CONSUMES the handle (single `Box::from_raw`). The
/// caller MUST set its pointer to NULL and never pass it to `tom_node_stop` /
/// `tom_node_free` again (double-free = UB). The Swift wrapper enforces this:
/// `stop()`/`forceReset()` are `guard let h = handle` + `handle = nil`, on an
/// actor (serialized) — so the second call is a no-op. There is therefore a
/// single teardown path; `stop` and `free` differ only by `graceful`.
fn detached_teardown(handle_box: Box<TomNodeHandle>, graceful: bool) {
    std::thread::spawn(move || {
        if graceful {
            // Signal the loop to break + persist state (bounded) before forcing.
            handle_box.runtime.block_on(async {
                if let Some(runtime_handle) = handle_box.handle.lock().await.take() {
                    tracing::info!("Shutting down TOM protocol node...");
                    let _ = tokio::time::timeout(
                        std::time::Duration::from_secs(3),
                        runtime_handle.shutdown(),
                    )
                    .await;
                }
            });
        }
        let TomNodeHandle { runtime, .. } = *handle_box;
        runtime.shutdown_timeout(std::time::Duration::from_secs(3));
        tracing::info!(graceful, "node teardown done (background)");
    });
}

/// Free a TomNodeHandle without graceful shutdown (e.g. forceReset after OS suspend).
///
/// # Safety
/// * `handle` must be a valid pointer returned by `tom_node_create()` and not
///   already freed/stopped (see `detached_teardown` ownership contract).
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_free(handle: *mut TomNodeHandle) {
    if handle.is_null() {
        return;
    }
    detached_teardown(unsafe { Box::from_raw(handle) }, false);
}

/// Send a 1-1 message to a peer
///
/// # Arguments
/// * `handle` - Opaque handle
/// * `target_id` - Recipient NodeId (hex string)
/// * `payload` - Raw bytes
/// * `payload_len` - Length of payload
///
/// # Returns
/// * 0 on success
/// * -1 on failure
///
/// # Safety
/// * `handle` must be valid
/// * `target_id` must be a valid null-terminated C string
/// * `payload` must be a valid pointer of length `payload_len`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_send_message(
    handle: *const TomNodeHandle,
    target_id: *const c_char,
    payload: *const u8,
    payload_len: usize,
) -> i32 {
    if handle.is_null() {
        return -1;
    }

    let handle_ref = unsafe { &*handle };

    let target_str = match unsafe { cstr_opt(target_id) } {
        Some(s) => s,
        None => {
            tracing::error!("tom_node_send_message: null or non-UTF-8 target_id");
            return -1;
        }
    };

    let target_node_id = match target_str.parse() {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Invalid NodeId: {}", e);
            return -1;
        }
    };

    // Null-safe payload read: a null pointer with len 0 is a legitimate empty
    // message (Swift's empty Data bridges to null), but a null pointer with a
    // non-zero len is a caller bug — reject it instead of risking UB.
    let payload_vec = if payload_len == 0 {
        Vec::new()
    } else if payload.is_null() {
        tracing::error!("tom_node_send_message: null payload with non-zero len");
        return -1;
    } else {
        unsafe { std::slice::from_raw_parts(payload, payload_len) }.to_vec()
    };

    let handle_clone = handle_ref.handle.clone();
    match bounded_block_on(&handle_ref.runtime, FFI_CALL_BUDGET, async move {
        if let Some(runtime_handle) = handle_clone.lock().await.as_ref() {
            match runtime_handle.send_message(target_node_id, payload_vec).await {
                Ok(_) => {
                    tracing::debug!("Message sent to {}", target_str);
                    0
                }
                Err(e) => {
                    tracing::error!("Failed to send message: {}", e);
                    -1
                }
            }
        } else {
            tracing::error!("Node not started");
            -1
        }
    }) {
        Some(rc) => rc,
        None => {
            *lock_recover(&handle_ref.last_error) = Some("transport occupé — runtime gelé, réessayer".to_string());
            -1
        }
    }
}

/// Create a new group
///
/// # Arguments
/// * `handle` - Opaque handle
/// * `group_config_json` - JSON with name, hub_relay_id, initial_members, invite_only
///
/// # Returns
/// * 0 on success (command sent to runtime)
/// * -1 on failure
///
/// # Note
/// * The group_id will be available via the `GroupCreated` event (poll events)
///
/// # Safety
/// * All pointers must be valid
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_create_group(
    handle: *const TomNodeHandle,
    group_config_json: *const c_char,
) -> i32 {
    if handle.is_null() {
        return -1;
    }

    let handle_ref = unsafe { &*handle };

    let config_str = match unsafe { cstr_opt(group_config_json) } {
        Some(s) => s,
        None => return -1,
    };

    let group_config: GroupConfigFFI = match serde_json::from_str(config_str) {
        Ok(c) => c,
        Err(e) => {
            tracing::error!("Invalid group config JSON: {}", e);
            return -1;
        }
    };

    let handle_clone = handle_ref.handle.clone();
    match bounded_block_on(&handle_ref.runtime, FFI_CALL_BUDGET, async move {
        if let Some(runtime_handle) = handle_clone.lock().await.as_ref() {
            if group_config.invite_only {
                runtime_handle
                    .create_group_invite_only(
                        group_config.name,
                        group_config.hub_relay_id,
                        group_config.initial_members,
                    )
                    .await
            } else {
                runtime_handle
                    .create_group(
                        group_config.name,
                        group_config.hub_relay_id,
                        group_config.initial_members,
                    )
                    .await
            }
        } else {
            Err(tom_protocol::TomProtocolError::InvalidEnvelope {
                reason: "Node not started".into(),
            })
        }
    }) {
        Some(result) => match result {
            Ok(_) => {
                tracing::debug!("Group creation command sent");
                0
            }
            Err(e) => {
                tracing::error!("Failed to create group: {}", e);
                -1
            }
        },
        None => {
            *lock_recover(&handle_ref.last_error) = Some("transport occupé — runtime gelé, réessayer".to_string());
            -1
        }
    }
}

/// Send a message to a group
///
/// # Arguments
/// * `handle` - Opaque handle
/// * `group_id` - Group ID (hex string)
/// * `text` - Message text
///
/// # Returns
/// * 0 on success
/// * -1 on failure
///
/// # Safety
/// * All pointers must be valid null-terminated C strings
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_send_group_message(
    handle: *const TomNodeHandle,
    group_id: *const c_char,
    text: *const c_char,
) -> i32 {
    if handle.is_null() {
        return -1;
    }

    let handle_ref = unsafe { &*handle };

    let group_id_str = match unsafe { cstr_opt(group_id) } {
        Some(s) => s,
        None => return -1,
    };

    let text_str = match unsafe { cstr_opt(text) } {
        Some(s) => s,
        None => return -1,
    };

    // GroupId is a newtype wrapper around String
    let group_id_parsed = tom_protocol::group::GroupId(group_id_str.to_string());

    let handle_clone = handle_ref.handle.clone();
    match bounded_block_on(&handle_ref.runtime, FFI_CALL_BUDGET, async move {
        if let Some(runtime_handle) = handle_clone.lock().await.as_ref() {
            match runtime_handle
                .send_group_message(group_id_parsed, text_str.to_string())
                .await
            {
                Ok(_) => 0,
                Err(e) => {
                    tracing::error!("Failed to send group message: {}", e);
                    -1
                }
            }
        } else {
            -1
        }
    }) {
        Some(rc) => rc,
        None => {
            *lock_recover(&handle_ref.last_error) = Some("transport occupé — runtime gelé, réessayer".to_string());
            -1
        }
    }
}

/// Accept a pending group invitation.
///
/// # Arguments
/// * `handle` - Opaque handle
/// * `group_id` - Group ID string of the pending invite
///
/// # Returns
/// * 0 on success, -1 on failure
///
/// # Safety
/// * All pointers must be valid null-terminated C strings
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_accept_group_invite(
    handle: *const TomNodeHandle,
    group_id: *const c_char,
) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let handle_ref = unsafe { &*handle };
    let group_id_str = match unsafe { cstr_opt(group_id) } {
        Some(s) => s,
        None => return -1,
    };
    let gid = tom_protocol::group::GroupId(group_id_str.to_string());

    let handle_clone = handle_ref.handle.clone();
    match bounded_block_on(&handle_ref.runtime, FFI_CALL_BUDGET, async move {
        if let Some(runtime_handle) = handle_clone.lock().await.as_ref() {
            match runtime_handle.accept_invite(gid).await {
                Ok(_) => 0,
                Err(e) => {
                    tracing::error!("Failed to accept group invite: {}", e);
                    -1
                }
            }
        } else {
            -1
        }
    }) {
        Some(rc) => rc,
        None => {
            *lock_recover(&handle_ref.last_error) = Some("transport occupé — runtime gelé, réessayer".to_string());
            -1
        }
    }
}

/// List the groups this node is a member of (or hosts).
///
/// # Returns
/// * JSON array: `[{"group_id":"...","name":"...","members":N}, ...]`
/// * Empty array `[]` if none
/// * NULL on error
///
/// # Safety
/// * Caller must free returned string with `tom_node_free_string()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_list_groups(handle: *const TomNodeHandle) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let handle_ref = unsafe { &*handle };

    let handle_clone = handle_ref.handle.clone();
    let json = match bounded_block_on(&handle_ref.runtime, FFI_CALL_BUDGET, async move {
        let guard = handle_clone.lock().await;
        let groups = if let Some(rh) = guard.as_ref() {
            rh.groups().await
        } else {
            Vec::new()
        };
        let items: Vec<serde_json::Value> = groups
            .iter()
            .map(|g| {
                serde_json::json!({
                    "group_id": g.group_id.as_ref(),
                    "name": g.name,
                    "members": g.members.len(),
                })
            })
            .collect();
        serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
    }) {
        Some(j) => j,
        None => {
            tracing::warn!("tom_node_list_groups: runtime gelé");
            "[]".to_string()
        }
    };

    match CString::new(json) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// List pending group invitations awaiting accept/decline.
///
/// # Returns
/// * JSON array: `[{"group_id":"...","group_name":"...","inviter":"..."}, ...]`
/// * Empty array `[]` if none
/// * NULL on error
///
/// # Safety
/// * Caller must free returned string with `tom_node_free_string()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_get_pending_invites(
    handle: *const TomNodeHandle,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let handle_ref = unsafe { &*handle };

    let handle_clone = handle_ref.handle.clone();
    let json = match bounded_block_on(&handle_ref.runtime, FFI_CALL_BUDGET, async move {
        let guard = handle_clone.lock().await;
        let invites = if let Some(rh) = guard.as_ref() {
            rh.pending_invites().await
        } else {
            Vec::new()
        };
        let items: Vec<serde_json::Value> = invites
            .iter()
            .map(|i| {
                serde_json::json!({
                    "group_id": i.group_id.as_ref(),
                    "group_name": i.group_name,
                    "inviter": i.inviter_id.to_string(),
                })
            })
            .collect();
        serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
    }) {
        Some(j) => j,
        None => {
            tracing::warn!("tom_node_get_pending_invites: runtime gelé");
            "[]".to_string()
        }
    };

    match CString::new(json) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Drain received group-message events (polled by Swift).
///
/// Returns the group messages delivered since the last call — including those
/// recovered via R13 offline gap-fill (SyncResponse). This is the observation
/// surface used to validate R13 on real devices.
///
/// ISOLATION: This function drains its own dedicated queue (group_message_queue),
/// separate from tom_node_receive_events(). No events are lost between callers.
///
/// # Returns
/// * JSON array: `[{"group_id":"...","from":"...","seq":N,"text":"...","encrypted":bool}, ...]`
/// * Empty array `[]` if none
/// * NULL on error
///
/// # Safety
/// * Caller must free returned string with `tom_node_free_string()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_receive_group_messages(
    handle: *const TomNodeHandle,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let handle_ref = unsafe { &*handle };

    // Drain transactionnel : la file n'est vidée que si l'appelant reçoit le
    // JSON (un timeout ne peut pas jeter le batch au dégel du runtime).
    let json = bounded_drain_json(
        &handle_ref.runtime,
        FFI_CALL_BUDGET,
        handle_ref.group_message_queue.clone(),
        |events: Vec<ProtocolEvent>| {
            let items: Vec<serde_json::Value> = events
                .into_iter()
                .filter_map(|evt| match evt {
                    ProtocolEvent::GroupMessageReceived { message } => Some(serde_json::json!({
                        "group_id": message.group_id.as_ref(),
                        "from": message.sender_id.to_string(),
                        "seq": message.seq,
                        "encrypted": message.encrypted,
                        "text": if message.text.is_empty() {
                            format!("[chiffré {}o]", message.ciphertext.len())
                        } else {
                            message.text.clone()
                        },
                    })),
                    _ => None,
                })
                .collect();
            serde_json::to_string(&items).unwrap_or_else(|_| "[]".to_string())
        },
    );

    match CString::new(json) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Receive messages (polled by Swift every ~500ms)
///
/// # Returns
/// * JSON array of messages: `[{"from": "...", "payload": "...", ...}, ...]`
/// * Empty array `[]` if no messages
/// * NULL on error
///
/// # Safety
/// * Caller must free returned string with `tom_node_free_string()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_receive_messages(
    handle: *const TomNodeHandle,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let handle_ref = unsafe { &*handle };

    // Drain transactionnel (cf. bounded_drain_json) : jamais de batch jeté.
    let messages_json = bounded_drain_json(
        &handle_ref.runtime,
        FFI_CALL_BUDGET,
        handle_ref.message_queue.clone(),
        |batch: Vec<DeliveredMessage>| {
            let ffi_batch: Vec<DeliveredMessageFFI> = batch.into_iter().map(Into::into).collect();
            match serde_json::to_string(&ffi_batch) {
                Ok(json) => json,
                Err(e) => {
                    tracing::error!("Failed to serialize messages: {}", e);
                    "[]".to_string()
                }
            }
        },
    );

    match CString::new(messages_json) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Drain protocol events from the runtime event queue.
///
/// Returns a JSON array of events that occurred since the last poll:
/// ```json
/// [
///   {
///     "type": "RolePromoted",
///     "ts_ms": 1234567890,
///     "node_id": "abc123...",
///     "score": 42.5
///   },
///   ...
/// ]
/// ```
///
/// # Returns
/// * JSON array of events: `[{"type": "...", ...}, ...]`
/// * Empty array `[]` if no events
/// * NULL on error
///
/// # Safety
/// * Caller must free returned string with `tom_node_free_string()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_receive_events(
    handle: *const TomNodeHandle,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let handle_ref = unsafe { &*handle };

    // Drain transactionnel (cf. bounded_drain_json) : jamais de batch jeté.
    let events_json = bounded_drain_json(
        &handle_ref.runtime,
        FFI_CALL_BUDGET,
        handle_ref.event_queue.clone(),
        |batch: Vec<ProtocolEvent>| {
            let ffi_batch: Vec<ProtocolEventFFI> = batch
                .into_iter()
                .map(ProtocolEventFFI::from_protocol_event)
                .collect();
            match serde_json::to_string(&ffi_batch) {
                Ok(json) => json,
                Err(e) => {
                    tracing::error!("Failed to serialize events: {}", e);
                    "[]".to_string()
                }
            }
        },
    );

    match CString::new(events_json) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get node status
///
/// # Returns
/// * JSON string with node_id, status, peers_count, groups_count
/// * NULL on error
///
/// # Safety
/// * Caller must free returned string with `tom_node_free_string()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_status(handle: *const TomNodeHandle) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let handle_ref = unsafe { &*handle };

    let node_id_clone = handle_ref.node_id.clone();
    let handle_clone = handle_ref.handle.clone();
    let local_role_clone = handle_ref.local_role.clone();
    let last_paths_clone = handle_ref.last_paths.clone();
    let configured_relay_url_clone = handle_ref.configured_relay_url.clone();
    let app_build_clone = handle_ref.app_build.clone();

    let status_json = match bounded_block_on(&handle_ref.runtime, FFI_CALL_BUDGET, async move {
        let node_id = node_id_clone.lock().await.clone();
        let guard = handle_clone.lock().await;

        let (status, peers_count, groups_count, relay_url_discovered) = if let Some(rh) = guard.as_ref() {
            let m = rh.metrics();
            // Pick the most recently refreshed discovered relay, if any
            let discovered = rh.get_known_relays().await;
            let relay_url = discovered
                .iter()
                .max_by_key(|e| e.refreshed_at)
                .map(|e| e.relay_url.to_string());
            ("Running", m.peers_known, m.groups_count, relay_url)
        } else {
            ("Stopped", 0, 0, None)
        };

        let local_role = local_role_clone.lock().await.clone();
        let (path_kind, path_rtt_ms, path_addr, paths_by_peer) = {
            let lp = last_paths_clone.lock().await;
            let paths_map = if lp.is_empty() {
                None
            } else {
                Some(lp.clone())
            };

            // Compute worst aggregate for compat: severity order UNKNOWN > RELAY > DIRECT,
            // and at equal severity use the path with max RTT. Fallback to RELAY.
            let (kind, rtt, addr) = if lp.is_empty() {
                ("RELAY".to_string(), 0, String::new())
            } else {
                let mut worst = PathInfoFFI {
                    kind: "DIRECT".to_string(),
                    rtt_ms: 0,
                    addr: String::new(),
                };
                for info in lp.values() {
                    let severity = match info.kind.as_str() {
                        "UNKNOWN" => 2,
                        "RELAY" => 1,
                        _ => 0, // DIRECT
                    };
                    let worst_severity = match worst.kind.as_str() {
                        "UNKNOWN" => 2,
                        "RELAY" => 1,
                        _ => 0,
                    };
                    // Replace if this is worse, or same severity with higher RTT
                    if severity > worst_severity || (severity == worst_severity && info.rtt_ms > worst.rtt_ms) {
                        worst = info.clone();
                    }
                }
                (worst.kind, worst.rtt_ms, worst.addr)
            };
            (kind, rtt, if addr.is_empty() { None } else { Some(addr) }, paths_map)
        };

        // Clock skew (optional, needs access to the runtime handle)
        let (clock_skew_ms, clock_skew_samples) = if let Some(rh) = guard.as_ref() {
            match rh.clock_skew().await {
                Some((median, samples)) => (median, samples as u64),
                None => (None, 0),
            }
        } else {
            (None, 0)
        };

        // relay_url_active = configured relay (explicit) or first discovered relay
        let configured = lock_recover(&configured_relay_url_clone).clone();
        let relay_url_active = configured.or(relay_url_discovered).unwrap_or_default();

        // Serialize through a typed struct (not a hand-rolled `format!`) so the
        // JSON is always valid/escaped and the key contract with the Swift
        // `TomNodeStatus` decoder stays locked by `types::tests`.
        let app_build = *lock_recover(&app_build_clone);
        let status_payload = NodeStatusFFI {
            node_id: node_id.unwrap_or_else(|| "unknown".to_string()),
            status: status.to_string(),
            peers_count,
            groups_count,
            local_role,
            path_kind,
            path_rtt_ms,
            path_addr,
            relay_url_active,
            clock_skew_ms,
            clock_skew_samples,
            app_build,
            paths_by_peer,
        };
        serde_json::to_string(&status_payload).unwrap_or_else(|_| "{}".to_string())
    }) {
        Some(j) => j,
        None => {
            tracing::warn!("tom_node_status: runtime gelé");
            "{}".to_string()
        }
    };

    match CString::new(status_json) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Issue a presence challenge toward a peer (L1-001).
///
/// The result arrives asynchronously: on acceptance the node updates its
/// presence stats (poll `tom_node_presence_stats()`) and logs the event.
/// No result at all means the peer is absent, lying, or below the
/// anti-Sybil gate — silent by design (no oracle).
///
/// # Returns
/// * 0 on success (command queued)
/// * -1 on failure (null/invalid handle or target, node not started)
///
/// # Safety
/// * `handle` must be a valid pointer returned by `tom_node_create()`
/// * `target_id` must be a valid NUL-terminated C string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_check_presence(
    handle: *const TomNodeHandle,
    target_id: *const c_char,
) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let handle_ref = unsafe { &*handle };

    let target_str = match unsafe { cstr_opt(target_id) } {
        Some(s) => s,
        None => {
            tracing::error!("tom_node_check_presence: null or non-UTF-8 target_id");
            return -1;
        }
    };
    let target_node_id = match target_str.parse() {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("tom_node_check_presence: invalid NodeId: {}", e);
            return -1;
        }
    };

    let handle_clone = handle_ref.handle.clone();
    match bounded_block_on(&handle_ref.runtime, FFI_CALL_BUDGET, async move {
        if let Some(runtime_handle) = handle_clone.lock().await.as_ref() {
            runtime_handle.check_presence(target_node_id).await;
            0
        } else {
            tracing::error!("tom_node_check_presence: node not started");
            -1
        }
    }) {
        Some(rc) => rc,
        None => {
            *lock_recover(&handle_ref.last_error) = Some("transport occupé — runtime gelé, réessayer".to_string());
            -1
        }
    }
}

/// Get L1-001 presence stats as JSON (see `PresenceStatsFFI` for the schema).
///
/// # Returns
/// * JSON C string (caller must free with `tom_node_free_string()`)
/// * NULL on null handle or serialization failure
///
/// # Safety
/// * `handle` must be a valid pointer returned by `tom_node_create()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_presence_stats(handle: *const TomNodeHandle) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let handle_ref = unsafe { &*handle };

    let presence_stats_clone = handle_ref.presence_stats.clone();
    let handle_clone = handle_ref.handle.clone();
    let json = match bounded_block_on(&handle_ref.runtime, FFI_CALL_BUDGET, async move {
        let (accepted_total, last_attester, last_latency_ms) =
            presence_stats_clone.lock().await.clone();

        let (window_count, seed_prefix) = {
            let guard = handle_clone.lock().await;
            if let Some(rh) = guard.as_ref() {
                match rh.presence_seed().await {
                    Some((seed, count)) => {
                        let prefix: String =
                            seed[..4].iter().map(|b| format!("{b:02x}")).collect();
                        (count as u64, prefix)
                    }
                    None => (0, String::new()),
                }
            } else {
                (0, String::new())
            }
        };

        let stats = PresenceStatsFFI {
            accepted_total,
            last_attester,
            last_latency_ms,
            window_count,
            seed_prefix,
        };
        serde_json::to_string(&stats).unwrap_or_else(|_| "{}".to_string())
    }) {
        Some(j) => j,
        None => {
            tracing::warn!("tom_node_presence_stats: runtime gelé");
            "{}".to_string()
        }
    };

    match CString::new(json) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Challenge many peers at once (L1-001 stress driving).
///
/// `targets_json` is a JSON array of NodeId strings, e.g. `["ab..","cd.."]`.
///
/// # Returns
/// * number of challenges queued (>= 0) on success
/// * -1 on failure (null/invalid handle or JSON, node not started)
///
/// # Safety
/// * `handle` must be valid; `targets_json` a valid NUL-terminated C string
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_check_presence_many(
    handle: *const TomNodeHandle,
    targets_json: *const c_char,
) -> i32 {
    if handle.is_null() {
        return -1;
    }
    let handle_ref = unsafe { &*handle };

    let json = match unsafe { cstr_opt(targets_json) } {
        Some(s) => s,
        None => return -1,
    };
    let target_strs: Vec<String> = match serde_json::from_str(json) {
        Ok(v) => v,
        Err(e) => {
            tracing::error!("tom_node_check_presence_many: bad JSON: {}", e);
            return -1;
        }
    };
    let mut targets = Vec::with_capacity(target_strs.len());
    for t in &target_strs {
        match t.parse() {
            Ok(id) => targets.push(id),
            Err(e) => {
                tracing::error!("tom_node_check_presence_many: invalid NodeId {t}: {e}");
                return -1;
            }
        }
    }

    let count = targets.len() as i32;
    let handle_clone = handle_ref.handle.clone();
    match bounded_block_on(&handle_ref.runtime, FFI_CALL_BUDGET, async move {
        if let Some(rh) = handle_clone.lock().await.as_ref() {
            rh.check_presence_many(targets).await;
            count
        } else {
            -1
        }
    }) {
        Some(rc) => rc,
        None => {
            *lock_recover(&handle_ref.last_error) = Some("transport occupé — runtime gelé, réessayer".to_string());
            -1
        }
    }
}

/// Get L1-001 full presence counters as JSON (see `PresenceMetricsFFI`).
///
/// # Returns
/// * JSON C string (caller must free with `tom_node_free_string()`)
/// * NULL on null handle, node not started, or serialization failure
///
/// # Safety
/// * `handle` must be a valid pointer returned by `tom_node_create()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_presence_metrics(handle: *const TomNodeHandle) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let handle_ref = unsafe { &*handle };

    let handle_clone = handle_ref.handle.clone();
    let json = bounded_block_on(&handle_ref.runtime, FFI_CALL_BUDGET, async move {
        let guard = handle_clone.lock().await;
        let rh = guard.as_ref()?;
        let metrics = rh.presence_metrics().await?;
        let ffi: PresenceMetricsFFI = metrics.into();
        serde_json::to_string(&ffi).ok()
    }).flatten();

    match json {
        Some(j) => match CString::new(j) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Get the last error message (after a function returned -1)
///
/// # Returns
/// * Error message as C string (caller must free with `tom_node_free_string()`)
/// * NULL if no error
///
/// # Safety
/// * `handle` must be a valid pointer returned by `tom_node_create()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_last_error(handle: *const TomNodeHandle) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let handle_ref = unsafe { &*handle };
    let error = lock_recover(&handle_ref.last_error).clone();

    match error {
        Some(msg) => match CString::new(msg) {
            Ok(c_str) => c_str.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Add a peer address (so this node can connect to it)
///
/// # Arguments
/// * `handle` - Opaque handle
/// * `peer_addr_json` - JSON with node_id, relay_url, direct_addrs
///   Example: {"node_id":"<hex>","relay_url":"http://1.2.3.4:3340","direct_addrs":["192.168.0.83:3340"]}
///   Only node_id is required. relay_url and direct_addrs are optional.
///
/// # Returns
/// * 0 on success, -1 on failure
///
/// # Safety
/// * All pointers must be valid
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_add_peer_addr(
    handle: *const TomNodeHandle,
    peer_addr_json: *const c_char,
) -> i32 {
    if handle.is_null() {
        return -1;
    }

    let handle_ref = unsafe { &*handle };

    let json_str = match unsafe { cstr_opt(peer_addr_json) } {
        Some(s) => s,
        None => return -1,
    };

    let addr_ffi: PeerAddrFFI = match serde_json::from_str(json_str) {
        Ok(a) => a,
        Err(e) => {
            tracing::error!("Invalid peer addr JSON: {}", e);
            return -1;
        }
    };

    let node_id: tom_protocol::types::NodeId = match addr_ffi.node_id.parse() {
        Ok(id) => id,
        Err(e) => {
            tracing::error!("Invalid node_id: {}", e);
            return -1;
        }
    };

    let mut endpoint_addr = tom_connect::EndpointAddr::new(*node_id.as_endpoint_id());

    if let Some(relay_url) = normalized_non_empty(addr_ffi.relay_url.as_deref()) {
        if let Ok(url) = relay_url.parse() {
            endpoint_addr = endpoint_addr.with_relay_url(url);
        }
    }

    if let Some(addrs) = &addr_ffi.direct_addrs {
        for addr_str in addrs {
            if let Ok(addr) = addr_str.parse() {
                endpoint_addr = endpoint_addr.with_ip_addr(addr);
            }
        }
    }

    let handle_clone = handle_ref.handle.clone();
    let node_id_str = addr_ffi.node_id.clone();
    match bounded_block_on(&handle_ref.runtime, FFI_CALL_BUDGET, async move {
        if let Some(runtime_handle) = handle_clone.lock().await.as_ref() {
            runtime_handle.add_peer_addr(endpoint_addr).await;
            tracing::info!("Added peer addr for {}", node_id_str);
            0
        } else {
            tracing::error!("Node not started");
            -1
        }
    }) {
        Some(rc) => rc,
        None => {
            *lock_recover(&handle_ref.last_error) = Some("transport occupé — runtime gelé, réessayer".to_string());
            -1
        }
    }
}

/// Get connected peers as JSON array of Node IDs
///
/// # Returns
/// * JSON array: ["<hex_id_1>", "<hex_id_2>", ...]
/// * Empty array "[]" if no peers
///
/// # Safety
/// * Caller must free returned string with `tom_node_free_string()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_connected_peers(
    handle: *const TomNodeHandle,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let handle_ref = unsafe { &*handle };

    let handle_clone = handle_ref.handle.clone();
    let peers_json = match bounded_block_on(&handle_ref.runtime, FFI_CALL_BUDGET, async move {
        if let Some(runtime_handle) = handle_clone.lock().await.as_ref() {
            let peers = runtime_handle.connected_peers().await;
            let peer_strings: Vec<String> = peers.iter().map(|p| p.to_string()).collect();
            serde_json::to_string(&peer_strings).unwrap_or_else(|_| "[]".to_string())
        } else {
            "[]".to_string()
        }
    }) {
        Some(j) => j,
        None => {
            tracing::warn!("tom_node_connected_peers: runtime gelé");
            "[]".to_string()
        }
    };

    match CString::new(peers_json) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get peers discovered via gossip/DHT as JSON array
///
/// # Returns
/// * JSON array: [{"node_id":"...","username":"...","source":"...","discovered_at":123}, ...]
/// * Empty array "[]" if no peers discovered yet
///
/// # Safety
/// * Caller must free returned string with `tom_node_free_string()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_discovered_peers(
    handle: *const TomNodeHandle,
) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }

    let handle_ref = unsafe { &*handle };

    let discovered_peers_clone = handle_ref.discovered_peers.clone();
    let peers_json = match bounded_block_on(&handle_ref.runtime, FFI_CALL_BUDGET, async move {
        // Clone under lock, then sort + serialize outside lock to reduce contention
        // with the background pump task.
        let mut peer_list: Vec<DiscoveredPeerFFI> = {
            let peers = discovered_peers_clone.lock().await;
            peers.values().cloned().collect()
        };

        // Stable UX: most recently discovered peers first, deterministic tie-break.
        peer_list.sort_by(|a, b| {
            b.discovered_at
                .cmp(&a.discovered_at)
                .then_with(|| a.node_id.cmp(&b.node_id))
        });

        serde_json::to_string(&peer_list).unwrap_or_else(|_| "[]".to_string())
    }) {
        Some(j) => j,
        None => {
            tracing::warn!("tom_node_discovered_peers: runtime gelé");
            "[]".to_string()
        }
    };

    match CString::new(peers_json) {
        Ok(c_str) => c_str.into_raw(),
        Err(_) => std::ptr::null_mut(),
    }
}

/// Get the best available relay URL for this node.
///
/// Priority: (1) configured relay URL, (2) most recently discovered relay via gossip.
/// Returns NULL when no relay is known yet — the runtime will still work via
/// N0 public relays if n0_discovery is enabled.
///
/// # Returns
/// * Relay URL as null-terminated C string (caller must free with `tom_node_free_string()`)
/// * NULL if no relay is known
///
/// # Safety
/// * `handle` must be a valid pointer returned by `tom_node_create()`
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_get_discovered_relay(handle: *const TomNodeHandle) -> *mut c_char {
    if handle.is_null() {
        return std::ptr::null_mut();
    }
    let handle_ref = unsafe { &*handle };

    let configured_relay_url_clone = handle_ref.configured_relay_url.clone();
    let handle_clone = handle_ref.handle.clone();
    let url = bounded_block_on(&handle_ref.runtime, FFI_CALL_BUDGET, async move {
        // Priority 1: configured relay URL (explicit, set at start time)
        let configured = lock_recover(&configured_relay_url_clone).clone();
        if configured.is_some() {
            return configured;
        }
        // Priority 2: most recently refreshed relay via gossip/DHT
        let guard = handle_clone.lock().await;
        if let Some(rh) = guard.as_ref() {
            let relays = rh.get_known_relays().await;
            relays.into_iter()
                .max_by_key(|e| e.refreshed_at)
                .map(|e| e.relay_url.to_string())
        } else {
            None
        }
    }).flatten();

    match url {
        Some(s) => match CString::new(s) {
            Ok(c) => c.into_raw(),
            Err(_) => std::ptr::null_mut(),
        },
        None => std::ptr::null_mut(),
    }
}

/// Free a string returned by FFI functions
///
/// # Safety
/// * `s` must be a valid pointer returned by `tom_node_*` functions
#[unsafe(no_mangle)]
pub unsafe extern "C" fn tom_node_free_string(s: *mut c_char) {
    if !s.is_null() {
        let _ = unsafe { CString::from_raw(s) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_node_lifecycle() {
        use std::ffi::CString;

        // Minimal config
        let node_config = NodeConfigFFI {
            relay_url: Some("http://127.0.0.1:3343".to_string()),
            n0_discovery: Some(false),
            identity_path: None,
        };
        let node_config_json = serde_json::to_string(&node_config).unwrap();
        let node_config_cstr = CString::new(node_config_json).unwrap();

        let runtime_config = RuntimeConfigFFI {
            username: "test_node".to_string(),
            encryption: Some(false),
            enable_dht: Some(false),
            relay_url: Some("http://127.0.0.1:3343".to_string()),
            identity_path: None,
            n0_discovery: Some(false),
            local_discovery: Some(true),
            data_dir: None,
            gossip_bootstrap_peers: vec![],
            enable_embedded_relay: Some(false),
            enable_embedded_relay_publication: Some(false),
            enable_transport_relay_discovery: Some(false),
            presence_contribution_min: None,
            presence_probe_interval_secs: None,
            app_build: None,
            relay_discovery_url: None,
            start_timeout_secs: None,
        };
        let runtime_config_json = serde_json::to_string(&runtime_config).unwrap();
        let runtime_config_cstr = CString::new(runtime_config_json).unwrap();

        unsafe {
            let handle = tom_node_create(node_config_cstr.as_ptr());
            assert!(!handle.is_null(), "tom_node_create should return valid handle");

            let start_result = tom_node_start(handle, runtime_config_cstr.as_ptr());
            assert_eq!(start_result, 0, "tom_node_start should succeed");

            // Give node time to bind
            std::thread::sleep(std::time::Duration::from_secs(1));

            let status_ptr = tom_node_status(handle);
            assert!(!status_ptr.is_null(), "tom_node_status should return valid pointer");

            let status_cstr = CStr::from_ptr(status_ptr);
            let status_str = status_cstr.to_str().unwrap();
            println!("Status: {}", status_str);
            assert!(status_str.contains("Running"));

            tom_node_free_string(status_ptr);
            tom_node_stop(handle);
        }
    }

    /// Contrat anti-zombie du start : sur réseau dégradé (ici un serveur
    /// discovery qui accepte la connexion puis ne répond jamais), tom_node_start
    /// doit rendre la main dans le budget start_timeout_secs avec -1 +
    /// last_error — jamais bloquer l'appelant (l'actor Swift) indéfiniment.
    /// Le même handle doit ensuite pouvoir re-démarrer proprement quand le
    /// réseau va mieux (pas d'état résiduel du start expiré).
    #[test]
    fn start_expires_on_stalled_network_then_recovers() {
        use std::io::Read as _;
        use std::net::TcpListener;
        use std::time::{Duration, Instant};

        // Serveur « réseau dégradé » : accepte, lit la requête, ne répond jamais.
        // reqwest (timeout client 3 s) garde le bind suspendu ~3 s > budget 1 s.
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            if let Ok((mut conn, _)) = listener.accept() {
                let mut buf = [0u8; 1024];
                let _ = conn.set_read_timeout(Some(Duration::from_secs(10)));
                let _ = conn.read(&mut buf);
                std::thread::sleep(Duration::from_secs(5));
            }
        });

        let h = make_unstarted_handle();
        assert!(!h.is_null());

        let stalled_cfg = CString::new(format!(
            r#"{{"username":"stalltest","encryption":false,"enable_dht":false,
                "n0_discovery":false,"local_discovery":false,
                "enable_embedded_relay":false,"enable_embedded_relay_publication":false,
                "enable_transport_relay_discovery":false,
                "relay_url":"http://127.0.0.1:3343",
                "relay_discovery_url":"http://127.0.0.1:{port}",
                "start_timeout_secs":1}}"#
        ))
        .unwrap();

        let t0 = Instant::now();
        let rc = unsafe { tom_node_start(h, stalled_cfg.as_ptr()) };
        let elapsed = t0.elapsed();

        assert_eq!(rc, -2, "un start suspendu par le réseau doit expirer (-2), pas réussir");
        assert!(
            elapsed < Duration::from_secs(3),
            "le start doit rendre la main au budget (1s), pas attendre le stall complet (~3s) — mesuré {elapsed:?}"
        );

        let err_ptr = unsafe { tom_node_last_error(h) };
        assert!(!err_ptr.is_null(), "un start expiré doit poser last_error");
        let err = unsafe { CStr::from_ptr(err_ptr) }.to_string_lossy().into_owned();
        unsafe { tom_node_free_string(err_ptr) };
        assert!(
            err.contains("expiré"),
            "last_error doit expliquer l'expiration réseau, reçu : {err}"
        );

        // Re-start sur le MÊME handle, réseau « revenu » (pas de discovery URL).
        let ok_cfg = CString::new(
            r#"{"username":"stalltest","encryption":false,"enable_dht":false,
                "n0_discovery":false,"local_discovery":false,
                "enable_embedded_relay":false,"enable_embedded_relay_publication":false,
                "enable_transport_relay_discovery":false,
                "relay_url":"http://127.0.0.1:3343"}"#,
        )
        .unwrap();
        let rc2 = unsafe { tom_node_start(h, ok_cfg.as_ptr()) };
        assert_eq!(rc2, 0, "le handle doit rester réutilisable après un start expiré");

        let status_ptr = unsafe { tom_node_status(h) };
        assert!(!status_ptr.is_null());
        let status = unsafe { CStr::from_ptr(status_ptr) }.to_string_lossy().into_owned();
        unsafe { tom_node_free_string(status_ptr) };
        assert!(status.contains("Running"), "status après re-start : {status}");

        unsafe { tom_node_stop(h) };
    }

    /// Un runtime tokio ENTIÈREMENT gelé (tous les workers bloqués — famille
    /// deadlock transport, prouvé par os.log iPad 2026-07-17 : stall de
    /// 5 min 21 s pendant lequel le timeout tokio n'a JAMAIS tiré) ne doit
    /// pas geler l'appelant : le contrat -2 doit être tenu par une primitive
    /// OS côté thread appelant, pas par le timer du runtime qu'on borne.
    #[test]
    fn start_returns_minus2_even_when_runtime_is_wedged() {
        use std::time::Duration;

        let h = make_unstarted_handle();
        assert!(!h.is_null());

        // Sature TOUS les workers : chaque tâche park() son thread pour
        // toujours → plus aucun poll possible, timer tokio mort.
        {
            let handle_ref = unsafe { &*h };
            for _ in 0..64 {
                handle_ref.runtime.spawn(async {
                    std::thread::park();
                });
            }
        }
        std::thread::sleep(Duration::from_millis(300));

        let cfg = CString::new(
            r#"{"username":"wedge","encryption":false,"enable_dht":false,
                "n0_discovery":false,"local_discovery":false,
                "enable_embedded_relay":false,"enable_embedded_relay_publication":false,
                "enable_transport_relay_discovery":false,
                "relay_url":"http://127.0.0.1:3343",
                "start_timeout_secs":1}"#,
        )
        .unwrap();

        // Le start part sur un thread jetable : si le contrat est violé (gel),
        // le test échoue proprement au lieu de bloquer la suite.
        let (tx, rx) = std::sync::mpsc::channel();
        let addr = h as usize;
        std::thread::spawn(move || {
            let rc = unsafe { tom_node_start(addr as *mut TomNodeHandle, cfg.as_ptr()) };
            let _ = tx.send(rc);
        });

        match rx.recv_timeout(Duration::from_secs(10)) {
            Ok(rc) => assert_eq!(rc, -2, "runtime gelé → le start doit rendre le contrat -2"),
            Err(_) => panic!(
                "tom_node_start a GELÉ sur un runtime wedgé — le budget doit être tenu côté appelant (recv_timeout), pas par le timer tokio"
            ),
        }
        // Pas de tom_node_stop : le runtime est volontairement gelé, son
        // teardown resterait coincé — le process de test s'en charge.
    }

    /// Le drain transactionnel ne jette JAMAIS un batch : runtime gelé →
    /// l'appelant reçoit "[]" mais la file garde ses éléments pour le
    /// prochain poll (avant : la tâche tardive drainait dans le vide).
    #[test]
    fn bounded_drain_keeps_batch_when_runtime_wedged() {
        use std::time::Duration;

        let rt = tokio::runtime::Runtime::new().unwrap();
        for _ in 0..64 {
            rt.spawn(async {
                std::thread::park();
            });
        }
        std::thread::sleep(Duration::from_millis(200));

        let queue = Arc::new(Mutex::new(VecDeque::from(vec![1u8, 2, 3])));
        let json = bounded_drain_json(&rt, Duration::from_millis(300), queue.clone(), |v: Vec<u8>| {
            format!("[{}]", v.len())
        });
        assert_eq!(json, "[]", "runtime gelé → réponse vide, sans erreur");

        let q = queue.try_lock().expect("la file ne doit pas être tenue");
        assert_eq!(q.len(), 3, "le batch doit être conservé, pas drainé dans le vide");
        drop(q);
        // Runtime volontairement gelé : son Drop attendrait les workers parkés.
        std::mem::forget(rt);
    }

    #[test]
    fn normalized_non_empty_discards_blank_strings() {
        assert_eq!(normalized_non_empty(None), None);
        assert_eq!(normalized_non_empty(Some("   \n\t  ")), None);
        assert_eq!(normalized_non_empty(Some(" http://127.0.0.1:3340 ")), Some("http://127.0.0.1:3340".into()));
    }

    #[test]
    fn parse_gossip_bootstrap_peers_ignores_blank_entries() {
        let peers = vec![
            "   ".to_string(),
            "4e28f4706e0dcb01f13d74a9ea00d3bdfc62490c2f4a91f7cb8b14bed6a45814".to_string(),
            "\n".to_string(),
        ];

        let parsed = parse_gossip_bootstrap_peers(&peers);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].to_string(), "4e28f4706e0dcb01f13d74a9ea00d3bdfc62490c2f4a91f7cb8b14bed6a45814");
    }

    // ──────────────────────────────────────────────────────────────────────
    // FFI safety suite — null pointers, invalid input, string ownership.
    // These exercise the C ABI surface that powers every Apple app.
    // ──────────────────────────────────────────────────────────────────────

    const VALID_NODE_ID: &str =
        "4e28f4706e0dcb01f13d74a9ea00d3bdfc62490c2f4a91f7cb8b14bed6a45814";

    /// Create a valid handle WITHOUT starting it (no bind, no network).
    fn make_unstarted_handle() -> *mut TomNodeHandle {
        let cfg = NodeConfigFFI {
            relay_url: None,
            n0_discovery: Some(false),
            identity_path: None,
        };
        let json = serde_json::to_string(&cfg).unwrap();
        let cstr = CString::new(json).unwrap();
        unsafe { tom_node_create(cstr.as_ptr()) }
    }

    #[test]
    fn create_rejects_null_config() {
        let h = unsafe { tom_node_create(std::ptr::null()) };
        assert!(h.is_null(), "null config_json must not crash, must return null");
    }

    #[test]
    fn create_rejects_invalid_json() {
        let cstr = CString::new("{not valid json").unwrap();
        let h = unsafe { tom_node_create(cstr.as_ptr()) };
        assert!(h.is_null());
    }

    #[test]
    fn create_rejects_non_utf8_config() {
        // 0xFF is an invalid UTF-8 start byte; CString accepts it (no interior nul).
        let cstr = CString::new(vec![0xffu8, 0x28]).unwrap();
        let h = unsafe { tom_node_create(cstr.as_ptr()) };
        assert!(h.is_null(), "non-UTF-8 config must return null, not panic/UB");
    }

    #[test]
    fn null_handle_calls_are_safe() {
        let dummy = CString::new("x").unwrap();
        unsafe {
            // i32-returning functions → -1 on null handle
            assert_eq!(tom_node_start(std::ptr::null_mut(), dummy.as_ptr()), -1);
            assert_eq!(
                tom_node_send_message(std::ptr::null(), dummy.as_ptr(), std::ptr::null(), 0),
                -1
            );
            assert_eq!(tom_node_create_group(std::ptr::null(), dummy.as_ptr()), -1);
            assert_eq!(
                tom_node_send_group_message(std::ptr::null(), dummy.as_ptr(), dummy.as_ptr()),
                -1
            );
            assert_eq!(tom_node_add_peer_addr(std::ptr::null(), dummy.as_ptr()), -1);

            // string-returning functions → null on null handle
            assert!(tom_node_receive_messages(std::ptr::null()).is_null());
            assert!(tom_node_status(std::ptr::null()).is_null());
            assert!(tom_node_last_error(std::ptr::null()).is_null());
            assert!(tom_node_connected_peers(std::ptr::null()).is_null());
            assert!(tom_node_discovered_peers(std::ptr::null()).is_null());
            assert!(tom_get_discovered_relay(std::ptr::null()).is_null());

            // void functions on null → no-op, no crash
            tom_node_free(std::ptr::null_mut());
            tom_node_stop(std::ptr::null_mut());
            tom_node_free_string(std::ptr::null_mut());
        }
    }

    #[test]
    fn unstarted_handle_status_is_stopped() {
        let h = make_unstarted_handle();
        assert!(!h.is_null());
        unsafe {
            let p = tom_node_status(h);
            assert!(!p.is_null());
            let s = CStr::from_ptr(p).to_str().unwrap().to_string();
            tom_node_free_string(p);
            assert!(s.contains("\"status\":\"Stopped\""), "got: {s}");
            assert!(s.contains("\"peers_count\":0"), "got: {s}");
            tom_node_free(h);
        }
    }

    #[test]
    fn unstarted_handle_queues_are_empty() {
        let h = make_unstarted_handle();
        assert!(!h.is_null());
        unsafe {
            for ptr in [
                tom_node_receive_messages(h),
                tom_node_connected_peers(h),
                tom_node_discovered_peers(h),
            ] {
                assert!(!ptr.is_null());
                let s = CStr::from_ptr(ptr).to_str().unwrap().to_string();
                tom_node_free_string(ptr);
                assert_eq!(s, "[]");
            }
            // No error set yet → null
            assert!(tom_node_last_error(h).is_null());
            // No relay configured or discovered yet → null
            assert!(tom_get_discovered_relay(h).is_null());
            tom_node_free(h);
        }
    }

    #[test]
    fn send_message_null_target_is_rejected() {
        let h = make_unstarted_handle();
        let payload = [1u8, 2, 3];
        let rc =
            unsafe { tom_node_send_message(h, std::ptr::null(), payload.as_ptr(), payload.len()) };
        assert_eq!(rc, -1);
        unsafe { tom_node_free(h) };
    }

    #[test]
    fn send_message_null_payload_nonzero_len_is_rejected() {
        let h = make_unstarted_handle();
        let target = CString::new(VALID_NODE_ID).unwrap();
        let rc = unsafe { tom_node_send_message(h, target.as_ptr(), std::ptr::null(), 5) };
        assert_eq!(rc, -1, "null payload with non-zero len must be rejected, not UB");
        unsafe { tom_node_free(h) };
    }

    #[test]
    fn send_message_invalid_nodeid_is_rejected() {
        let h = make_unstarted_handle();
        let target = CString::new("not-a-valid-node-id").unwrap();
        let payload = [7u8];
        let rc =
            unsafe { tom_node_send_message(h, target.as_ptr(), payload.as_ptr(), payload.len()) };
        assert_eq!(rc, -1);
        unsafe { tom_node_free(h) };
    }

    #[test]
    fn send_message_valid_target_but_unstarted_is_rejected() {
        // Valid NodeId, valid payload, but node not started → no runtime handle.
        let h = make_unstarted_handle();
        let target = CString::new(VALID_NODE_ID).unwrap();
        let payload = [9u8, 9, 9];
        let rc =
            unsafe { tom_node_send_message(h, target.as_ptr(), payload.as_ptr(), payload.len()) };
        assert_eq!(rc, -1);
        unsafe { tom_node_free(h) };
    }

    #[test]
    fn create_group_null_and_invalid_json_rejected() {
        let h = make_unstarted_handle();
        unsafe {
            assert_eq!(tom_node_create_group(h, std::ptr::null()), -1);
            let bad = CString::new("{bad").unwrap();
            assert_eq!(tom_node_create_group(h, bad.as_ptr()), -1);
            tom_node_free(h);
        }
    }

    #[test]
    fn create_group_unstarted_is_rejected() {
        let h = make_unstarted_handle();
        let cfg = format!(
            r#"{{"name":"g","hub_relay_id":"{id}","initial_members":["{id}"],"invite_only":false}}"#,
            id = VALID_NODE_ID
        );
        let cstr = CString::new(cfg).unwrap();
        let rc = unsafe { tom_node_create_group(h, cstr.as_ptr()) };
        assert_eq!(rc, -1);
        unsafe { tom_node_free(h) };
    }

    #[test]
    fn send_group_message_null_args_rejected() {
        let h = make_unstarted_handle();
        let ok = CString::new("group-1").unwrap();
        unsafe {
            assert_eq!(tom_node_send_group_message(h, std::ptr::null(), ok.as_ptr()), -1);
            assert_eq!(tom_node_send_group_message(h, ok.as_ptr(), std::ptr::null()), -1);
            tom_node_free(h);
        }
    }

    #[test]
    fn add_peer_addr_null_invalid_and_bad_id_rejected() {
        let h = make_unstarted_handle();
        unsafe {
            assert_eq!(tom_node_add_peer_addr(h, std::ptr::null()), -1);
            let bad = CString::new("{nope").unwrap();
            assert_eq!(tom_node_add_peer_addr(h, bad.as_ptr()), -1);
            let bad_id = CString::new(r#"{"node_id":"xxx"}"#).unwrap();
            assert_eq!(tom_node_add_peer_addr(h, bad_id.as_ptr()), -1);
            tom_node_free(h);
        }
    }

    #[test]
    fn free_string_after_status_does_not_crash() {
        let h = make_unstarted_handle();
        unsafe {
            let p = tom_node_status(h);
            assert!(!p.is_null());
            tom_node_free_string(p);
            tom_node_free(h);
        }
    }

    // ── JSON wire contract between Swift and Rust ──────────────────────────

    #[test]
    fn runtime_config_deserializes_swift_json() {
        let json = r#"{"username":"alice","encryption":true,"enable_dht":false,"relay_url":"http://127.0.0.1:3340","n0_discovery":false,"local_discovery":true}"#;
        let cfg: RuntimeConfigFFI = serde_json::from_str(json).unwrap();
        assert_eq!(cfg.username, "alice");
        assert_eq!(cfg.encryption, Some(true));
        assert_eq!(cfg.enable_dht, Some(false));
        assert!(cfg.gossip_bootstrap_peers.is_empty());
    }

    #[test]
    fn peer_addr_deserializes_minimal_and_full() {
        let min: PeerAddrFFI = serde_json::from_str(r#"{"node_id":"abc"}"#).unwrap();
        assert_eq!(min.node_id, "abc");
        assert!(min.relay_url.is_none());
        assert!(min.direct_addrs.is_none());

        let full: PeerAddrFFI = serde_json::from_str(
            r#"{"node_id":"abc","relay_url":"http://x:3340","direct_addrs":["192.168.0.1:3340"]}"#,
        )
        .unwrap();
        assert_eq!(full.relay_url.as_deref(), Some("http://x:3340"));
        assert_eq!(full.direct_addrs.as_ref().unwrap().len(), 1);
    }
}
