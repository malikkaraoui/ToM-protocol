use crate::envelope::MessageEnvelope;
use crate::protocol::{spawn_inbound_stream_reader, spawn_path_watcher, ChunkBuffers};
use crate::{NodeId, PathEvent, TomTransportError};

use tom_connect::endpoint::Connection;
use tom_connect::{Endpoint, EndpointAddr};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, Mutex};

/// Caches QUIC connections per peer. First `send()` triggers connect,
/// subsequent sends reuse the cached connection.
pub(crate) struct ConnectionPool {
    endpoint: Endpoint,
    connections: Mutex<HashMap<NodeId, Connection>>,
    /// Connexions ENTRANTES (acceptées) — un pair qui nous a dialés. Sans ce
    /// registre, `connected_peers()` ne voyait QUE le pool sortant → un nœud
    /// relancé, joint par ses pairs, affichait 0 pair connecté alors qu'il
    /// l'était (connexion établie en ~0.2 s, mais invisible). Registre séparé
    /// pour ne pas entrer en conflit avec la réutilisation des sorties.
    inbound: Mutex<HashMap<NodeId, Connection>>,
    addresses: Mutex<HashMap<NodeId, EndpointAddr>>,
    alpn: Vec<u8>,
    /// Default relay URLs to include when no address is stored for a peer.
    /// Used when n0 discovery is disabled — the pool will try each relay in
    /// order before failing the connection attempt.
    default_relay_urls: Mutex<Vec<tom_connect::RelayUrl>>,
    /// Path events for outbound connections (same channel as the accept side).
    path_event_tx: broadcast::Sender<PathEvent>,
    /// Canaux de RÉCEPTION (mêmes que le handler d'accept) : permettent de lire
    /// les streams entrants sur les connexions SORTANTES aussi (#46), pour que
    /// UNE connexion par paire suffise dans les deux sens.
    incoming_tx: mpsc::Sender<(NodeId, MessageEnvelope)>,
    incoming_raw_tx: mpsc::Sender<(NodeId, Vec<u8>)>,
    chunk_buffers: Arc<ChunkBuffers>,
    max_message_size: usize,
}

impl ConnectionPool {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        endpoint: Endpoint,
        alpn: Vec<u8>,
        default_relay_urls: Vec<tom_connect::RelayUrl>,
        path_event_tx: broadcast::Sender<PathEvent>,
        incoming_tx: mpsc::Sender<(NodeId, MessageEnvelope)>,
        incoming_raw_tx: mpsc::Sender<(NodeId, Vec<u8>)>,
        chunk_buffers: Arc<ChunkBuffers>,
        max_message_size: usize,
    ) -> Self {
        Self {
            endpoint,
            connections: Mutex::new(HashMap::new()),
            inbound: Mutex::new(HashMap::new()),
            addresses: Mutex::new(HashMap::new()),
            alpn,
            default_relay_urls: Mutex::new(default_relay_urls),
            path_event_tx,
            incoming_tx,
            incoming_raw_tx,
            chunk_buffers,
            max_message_size,
        }
    }

    /// Plafond du registre des connexions entrantes. Généreux (un hub légitime
    /// peut fédérer beaucoup de spokes) mais fini : sans lui, un attaquant
    /// ouvrait N connexions QUIC (N NodeIds) maintenues vivantes par keep-alive,
    /// gonflant la map sans borne → OOM + `connected_peers()` O(N) à chaque
    /// refresh UI sur petit appareil (red-team #HAUT). Testé.
    const MAX_INBOUND: usize = 512;

    /// Enregistre une connexion ENTRANTE acceptée (voir champ `inbound`).
    ///
    /// Avant insertion : purge des entrées dont la connexion est FERMÉE (elles
    /// ne sont retirées que par `unregister_inbound` à la fin de l'accept-loop,
    /// ce qui peut traîner). Si le registre reste plein de connexions VIVANTES
    /// au-delà du plafond, la nouvelle est refusée — elle reste utilisable pour
    /// l'échange mais n'occupe pas d'entrée (au pire invisible dans le compteur,
    /// jamais un OOM).
    pub async fn register_inbound(&self, id: NodeId, conn: Connection) {
        let mut inbound = self.inbound.lock().await;
        if inbound.len() >= Self::MAX_INBOUND {
            inbound.retain(|_, c| c.close_reason().is_none());
        }
        if inbound.len() >= Self::MAX_INBOUND && !inbound.contains_key(&id) {
            tracing::warn!(
                "registre inbound plein ({} connexions vivantes) — connexion de {} non enregistrée",
                Self::MAX_INBOUND, id
            );
            return;
        }
        inbound.insert(id, conn);
    }

    /// Retire une connexion entrante (à la fermeture de l'accept-loop) —
    /// SEULEMENT si l'entrée du registre est bien CETTE connexion. Un pair
    /// qui redémarre re-dial : sa nouvelle connexion écrase l'ancienne dans
    /// la map (même NodeId), puis l'accept-loop de l'ANCIENNE se termine et
    /// venait supprimer l'entrée de la NOUVELLE pourtant vivante → le pair
    /// disparaissait de connected_peers pour toujours (0 pair affiché côté
    /// accepteur après un restart de flotte).
    pub async fn unregister_inbound(&self, id: &NodeId, conn: &Connection) {
        let mut inbound = self.inbound.lock().await;
        if inbound
            .get(id)
            .is_some_and(|current| current.stable_id() == conn.stable_id())
        {
            inbound.remove(id);
        }
    }

    /// Replace default relay URL candidates used when no peer address is known.
    pub async fn set_default_relay_urls(&self, relays: Vec<tom_connect::RelayUrl>) {
        *self.default_relay_urls.lock().await = relays;
    }

    /// Return current default relay URL candidates.
    pub async fn default_relay_urls(&self) -> Vec<tom_connect::RelayUrl> {
        self.default_relay_urls.lock().await.clone()
    }

    /// Store a known address for a peer.
    pub async fn add_addr(&self, id: NodeId, addr: EndpointAddr) {
        self.addresses.lock().await.insert(id, addr);
    }

    /// Get an existing connection or create a new one.
    pub async fn get_or_connect(
        &self,
        target: NodeId,
    ) -> Result<Connection, TomTransportError> {
        // #46c — RÉUTILISER une connexion ENTRANTE vivante EN PRIORITÉ. Quand
        // deux nœuds se dialent mutuellement, le remote_map (tom-connect) fusionne
        // les deux connexions QUIC et n'en garde qu'UNE ; NOTRE sortante peut être
        // celle abandonnée (mais `close_reason()` reste None, donc on ne peut pas
        // la détecter morte ici — on envoyait dans le vide → le pair ne recevait
        // jamais). La connexion ENTRANTE est celle que le PAIR a établie ET qu'il
        // LIT (via son accept-loop) → lui envoyer dessus garantit qu'il reçoit.
        // Nos propres réponses arrivent car on lit aussi les entrants sur nos
        // connexions (accept + spawn_inbound_stream_reader #46b). UNE connexion
        // par paire suffit alors dans les deux sens.
        {
            let inbound = self.inbound.lock().await;
            if let Some(conn) = inbound.get(&target) {
                if conn.close_reason().is_none() {
                    return Ok(conn.clone());
                }
            }
        }

        let mut conns = self.connections.lock().await;

        // Check if we have a cached connection that's still alive
        if let Some(conn) = conns.get(&target) {
            // connection.close_reason() returns Some if closed
            if conn.close_reason().is_none() {
                tracing::debug!("Reusing existing connection for {}", target);
                return Ok(conn.clone());
            }
            // Connection is dead, remove it
            tracing::debug!("Connection for {} is dead, removing", target);
            conns.remove(&target);
        } else {
            tracing::debug!("No cached connection for {}, will create new", target);
        }

        // Create new connection candidates — use stored address first, or
        // fallback to configured relay list (when n0 discovery is disabled).
        let stored_addr = {
            let addrs = self.addresses.lock().await;
            addrs.get(&target).cloned()
        };

        let default_relay_urls = self.default_relay_urls.lock().await.clone();

        let candidates: Vec<EndpointAddr> = if let Some(addr) = stored_addr {
            vec![addr]
        } else if !default_relay_urls.is_empty() {
            default_relay_urls
                .iter()
                .cloned()
                .map(|relay| EndpointAddr::new(*target.as_endpoint_id()).with_relay_url(relay))
                .collect()
        } else {
            vec![EndpointAddr::new(*target.as_endpoint_id())]
        };

        let mut last_err = None;
        let mut established = None;
        for addr in candidates {
            match self.endpoint.connect(addr, &self.alpn).await {
                Ok(conn) => {
                    established = Some(conn);
                    break;
                }
                Err(err) => {
                    last_err = Some(err);
                }
            }
        }

        let conn = if let Some(conn) = established {
            conn
        } else {
            return Err(TomTransportError::Connect {
                node_id: target,
                source: last_err
                    .expect("at least one connect attempt should have been made")
                    .into(),
            });
        };

        // Watcher aussi sur les connexions SORTANTES — sans lui, un nœud qui
        // ne fait que dialer n'émet aucun PathChanged (vue par pair asymétrique).
        spawn_path_watcher(&conn, target, self.path_event_tx.clone());

        // LECTURE des streams entrants sur cette connexion SORTANTE (#46) : une
        // connexion QUIC est bidirectionnelle, le pair peut nous envoyer des
        // messages dessus. Sans ça, seul le côté qui ACCEPTE lisait → collision
        // de dial mutuel = messages perdus. Voir spawn_inbound_stream_reader.
        spawn_inbound_stream_reader(
            conn.clone(),
            target,
            self.incoming_tx.clone(),
            self.incoming_raw_tx.clone(),
            self.chunk_buffers.clone(),
            self.max_message_size,
        );

        conns.insert(target, conn.clone());
        Ok(conn)
    }

    /// Remove a connection from the cache (e.g., after send failure).
    pub async fn remove(&self, target: &NodeId) {
        self.connections.lock().await.remove(target);
    }

    /// List all connected peers — connexions SORTANTES (pool) ET ENTRANTES
    /// (acceptées), dédupliquées. Compter les entrantes est indispensable :
    /// un nœud relancé est joint par ses pairs bien avant que ses propres
    /// dials aboutissent ; sans elles, il s'affichait « 0 pair » à tort.
    pub async fn connected_peers(&self) -> Vec<NodeId> {
        let mut set: std::collections::HashSet<NodeId> = std::collections::HashSet::new();
        {
            let conns = self.connections.lock().await;
            for (id, conn) in conns.iter() {
                if conn.close_reason().is_none() {
                    set.insert(*id);
                }
            }
        }
        {
            let inbound = self.inbound.lock().await;
            for (id, conn) in inbound.iter() {
                if conn.close_reason().is_none() {
                    set.insert(*id);
                }
            }
        }
        // Ordre STABLE (tri par id) : un HashSet renvoie un ordre différent à
        // chaque appel → la liste de pairs « sautait » à l'écran à chaque
        // rafraîchissement (1 s). L'UI ne doit pas re-trier, la source fait foi.
        let mut peers: Vec<NodeId> = set.into_iter().collect();
        peers.sort_by_key(|id| id.as_bytes());
        peers
    }
}
