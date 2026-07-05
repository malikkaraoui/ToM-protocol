use crate::envelope::MessageEnvelope;
use crate::path::{PathEvent, PathKind};
use crate::{NodeId, TomTransportError};

use tom_connect::endpoint::Connection;
use tom_connect::protocol::AcceptError;
use n0_future::StreamExt;
use n0_watcher::Watcher;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};

/// Write a length-prefixed message to a QUIC send stream.
pub(crate) async fn write_framed(
    send: &mut tom_connect::endpoint::SendStream,
    data: &[u8],
) -> Result<(), anyhow::Error> {
    let len = (data.len() as u32).to_be_bytes();
    send.write_all(&len).await?;
    send.write_all(data).await?;
    send.finish()?;
    Ok(())
}

/// Read a length-prefixed message from a QUIC receive stream.
pub(crate) async fn read_framed(
    recv: &mut tom_connect::endpoint::RecvStream,
    max_size: usize,
) -> Result<Vec<u8>, TomTransportError> {
    let mut len_buf = [0u8; 4];
    recv.read_exact(&mut len_buf)
        .await
        .map_err(|e| TomTransportError::Receive(e.into()))?;

    let len = u32::from_be_bytes(len_buf) as usize;
    if len > max_size {
        return Err(TomTransportError::MessageTooLarge {
            size: len,
            max: max_size,
        });
    }

    let mut buf = vec![0u8; len];
    recv.read_exact(&mut buf)
        .await
        .map_err(|e| TomTransportError::Receive(e.into()))?;

    Ok(buf)
}

// ── Chunking / segmentation des gros messages ──────────────────────────
//
// Le transport QUIC a un plafond pratique par stream (~256 Ko, fenêtre de
// flux) bien en-dessous de `max_message_size`. Pour envoyer des messages
// arbitrairement gros (10 Mo+), on segmente : un gros payload est découpé en
// chunks < plafond, chacun envoyé comme une frame normale, réassemblés à la
// réception. Transparent pour la couche protocole (elle voit le payload entier).

/// Marqueur d'une frame de chunk (préfixe des 4 premiers octets).
pub(crate) const CHUNK_MAGIC: &[u8; 4] = b"TCHK";
/// En-tête d'un chunk : MAGIC(4) + transfer_id(8) + total_chunks(4) + index(4).
pub(crate) const CHUNK_HEADER: usize = 20;
/// Taille des données par chunk (l'octet de frame reste < plafond ~256 Ko).
pub(crate) const CHUNK_PAYLOAD: usize = 200_000;
/// Au-delà de cette taille, un message est segmenté.
pub(crate) const CHUNK_THRESHOLD: usize = CHUNK_PAYLOAD;
/// Plafond dur de la taille réassemblée (anti-abus mémoire).
pub(crate) const MAX_REASSEMBLED: usize = 64 * 1024 * 1024;
/// Plafond dur du nombre de chunks d'un transfert. Anti-amplification : borne la
/// structure indépendamment du `total_chunks` annoncé par le pair (un pair
/// malveillant pourrait sinon annoncer 64M chunks pour forcer une allocation
/// géante). Légitime : 64 Mo / 200 Ko ≈ 336 chunks — 100k laisse une marge large.
pub(crate) const MAX_CHUNKS: u32 = 100_000;
/// Budget mémoire GLOBAL de réassemblage, tous transferts concurrents confondus.
/// Protège les appareils contraints (Apple TV, iPad) d'un flot de gros fichiers
/// simultanés qui, sans plafond global, cumulent N×64 Mo → OOM/jetsam.
pub(crate) const MAX_TOTAL_REASSEMBLY: usize = 128 * 1024 * 1024;
/// Nombre max de transferts concurrents PAR PAIR. Anti-spam : sans ça un pair
/// pourrait ouvrir des milliers de transferts partiels (chacun sous le plafond),
/// saturer le budget global et bloquer le réassemblage pour tout le monde.
pub(crate) const MAX_CONCURRENT_PER_PEER: usize = 16;
/// Durée d'INACTIVITÉ au-delà de laquelle un transfert partiel est purgé (zombie :
/// un pair envoie le 1er chunk et jamais la suite). Basé sur la dernière activité,
/// donc un transfert lent mais actif (chunks qui arrivent) n'est jamais purgé.
pub(crate) const REASSEMBLY_TTL: std::time::Duration = std::time::Duration::from_secs(30);

/// Buffer de réassemblage d'un transfert segmenté. Les chunks reçus sont stockés
/// dans une `BTreeMap` (clé = index) qui ne grandit qu'avec les données réellement
/// reçues — jamais de pré-allocation proportionnelle au `total_chunks` annoncé.
/// L'itération d'une `BTreeMap` est en ordre de clé croissant → concaténation
/// dans l'ordre des index sans tri explicite.
pub(crate) struct Reassembly {
    total_chunks: u32,
    chunks: std::collections::BTreeMap<u32, Vec<u8>>,
    bytes: usize,
    /// Dernier instant où un chunk a été ajouté — sert au TTL d'inactivité.
    last_activity: std::time::Instant,
}

pub(crate) type ChunkBuffers = std::sync::Mutex<
    std::collections::HashMap<(NodeId, u64), Reassembly>,
>;

/// Sérialise un chunk : MAGIC + transfer_id + total_chunks + index + data.
pub(crate) fn encode_chunk(
    transfer_id: u64,
    total_chunks: u32,
    index: u32,
    data: &[u8],
) -> Vec<u8> {
    let mut out = Vec::with_capacity(CHUNK_HEADER + data.len());
    out.extend_from_slice(CHUNK_MAGIC);
    out.extend_from_slice(&transfer_id.to_be_bytes());
    out.extend_from_slice(&total_chunks.to_be_bytes());
    out.extend_from_slice(&index.to_be_bytes());
    out.extend_from_slice(data);
    out
}

/// Traite une frame reçue : réassemble si c'est un chunk, sinon renvoie le
/// payload tel quel. Renvoie `Some(bytes)` quand un message complet est prêt.
fn reassemble(buffers: &ChunkBuffers, remote: NodeId, data: Vec<u8>) -> Option<Vec<u8>> {
    // Frame normale (pas un chunk) → message complet directement.
    if data.len() < CHUNK_HEADER || &data[0..4] != CHUNK_MAGIC {
        return Some(data);
    }
    let transfer_id = u64::from_be_bytes(data[4..12].try_into().ok()?);
    let total_chunks = u32::from_be_bytes(data[12..16].try_into().ok()?);
    let index = u32::from_be_bytes(data[16..20].try_into().ok()?);
    let payload = data[CHUNK_HEADER..].to_vec();

    if total_chunks == 0
        || total_chunks > MAX_CHUNKS
        || index >= total_chunks
        || total_chunks as usize > MAX_REASSEMBLED / 1.max(payload.len())
    {
        tracing::warn!("chunk invalide de {remote} (total={total_chunks}, idx={index}) — ignoré");
        return None;
    }

    let mut buffers = buffers.lock().unwrap_or_else(|e| e.into_inner());

    // Purge des transferts partiels ZOMBIES (inactifs > TTL) : sans ça, un pair
    // qui envoie le 1er chunk et jamais la suite occupe le budget indéfiniment.
    buffers.retain(|_, r| r.last_activity.elapsed() < REASSEMBLY_TTL);

    // Budget mémoire global : somme des octets de TOUS les transferts en vol.
    // On rejette le chunk si l'ajouter dépasserait le plafond global — protège
    // les appareils contraints d'un flot de gros fichiers concurrents.
    let total_in_flight: usize = buffers.values().map(|r| r.bytes).sum();
    if total_in_flight + payload.len() > MAX_TOTAL_REASSEMBLY {
        tracing::warn!(
            "budget réassemblage global dépassé ({total_in_flight}+{} > {MAX_TOTAL_REASSEMBLY}o) — chunk de {remote} rejeté",
            payload.len()
        );
        return None;
    }

    // Cap du nombre de transferts concurrents PAR PAIR (anti-spam de transfer_id).
    // Uniquement pour un NOUVEAU transfert ; un chunk d'un transfert déjà en cours
    // passe toujours.
    if !buffers.contains_key(&(remote, transfer_id)) {
        let per_peer = buffers.keys().filter(|(n, _)| *n == remote).count();
        if per_peer >= MAX_CONCURRENT_PER_PEER {
            tracing::warn!(
                "trop de transferts concurrents de {remote} ({per_peer} ≥ {MAX_CONCURRENT_PER_PEER}) — chunk rejeté"
            );
            return None;
        }
    }

    let entry = buffers.entry((remote, transfer_id)).or_insert_with(|| Reassembly {
        total_chunks,
        chunks: std::collections::BTreeMap::new(),
        bytes: 0,
        last_activity: std::time::Instant::now(),
    });

    // Cohérence : tous les chunks d'un transfert doivent annoncer le même total.
    if entry.total_chunks != total_chunks {
        tracing::warn!("total_chunks incohérent pour {transfer_id} de {remote} — abandon");
        buffers.remove(&(remote, transfer_id));
        return None;
    }

    if let std::collections::btree_map::Entry::Vacant(slot) = entry.chunks.entry(index) {
        entry.bytes += payload.len();
        slot.insert(payload);
    }
    // Marque l'activité (même un chunk dupliqué prouve que le transfert est vivant)
    // → évite la purge TTL d'un transfert lent mais actif.
    entry.last_activity = std::time::Instant::now();

    if entry.bytes > MAX_REASSEMBLED {
        tracing::warn!("transfert {transfer_id} de {remote} dépasse {MAX_REASSEMBLED}o — abandon");
        buffers.remove(&(remote, transfer_id));
        return None;
    }

    if entry.chunks.len() as u32 == entry.total_chunks {
        let entry = buffers.remove(&(remote, transfer_id))?;
        let mut full = Vec::with_capacity(entry.bytes);
        for chunk in entry.chunks.into_values() {
            full.extend_from_slice(&chunk);
        }
        tracing::debug!(
            "message réassemblé de {remote}: {} octets ({} chunks)",
            full.len(),
            total_chunks
        );
        return Some(full);
    }
    None
}

/// Internal state shared with the protocol handler.
pub(crate) struct HandlerState {
    pub incoming_tx: mpsc::Sender<(NodeId, MessageEnvelope)>,
    pub incoming_raw_tx: mpsc::Sender<(NodeId, Vec<u8>)>,
    pub path_event_tx: broadcast::Sender<PathEvent>,
    pub pool: Arc<crate::connection::ConnectionPool>,
    pub max_message_size: usize,
    pub chunk_buffers: Arc<ChunkBuffers>,
}

/// Protocol handler that accepts incoming ToM connections.
#[derive(Clone)]
pub(crate) struct TomProtocolHandler {
    pub state: Arc<HandlerState>,
}

impl std::fmt::Debug for TomProtocolHandler {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TomProtocolHandler").finish()
    }
}

impl tom_connect::protocol::ProtocolHandler for TomProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote = NodeId::from_endpoint_id(connection.remote_id());
        let state = self.state.clone();

        tracing::debug!("Accepted connection from {}", remote);

        // Auto-learn relay route: extract the relay URL from the incoming
        // connection's path info so replies can reach this peer without
        // manual seeding.
        {
            let mut paths = connection.paths();
            for path in paths.get().iter() {
                if let tom_base::TransportAddr::Relay(relay_url) = path.remote_addr() {
                    let addr = tom_connect::EndpointAddr::new(connection.remote_id())
                        .with_relay_url(relay_url.clone());
                    state.pool.add_addr(remote, addr).await;
                    tracing::debug!("Auto-learned relay route for {} via {}", remote, relay_url);
                    break;
                }
            }
        }

        // Spawn path watcher for this connection
        spawn_path_watcher(&connection, remote, state.path_event_tx.clone());

        // Accept loop: handle multiple bi-directional streams from this connection
        loop {
            let (mut send, mut recv) = match connection.accept_bi().await {
                Ok(streams) => streams,
                Err(_) => break, // Connection closed
            };

            let state = state.clone();
            tokio::spawn(async move {
                match read_framed(&mut recv, state.max_message_size).await {
                    Ok(data) => {
                        // Réassemblage : un chunk incomplet renvoie None (bufferisé) ;
                        // une frame normale ou le dernier chunk renvoie le message entier.
                        if let Some(full) = reassemble(&state.chunk_buffers, remote, data) {
                            match MessageEnvelope::from_bytes(&full) {
                                Ok(envelope) => {
                                    let _ = state.incoming_tx.send((remote, envelope)).await;
                                }
                                Err(_) => {
                                    // Not a valid envelope — deliver as raw
                                    let _ = state.incoming_raw_tx.send((remote, full)).await;
                                }
                            }
                        }
                        // Acknowledge receipt by closing our send stream
                        let _ = send.finish();
                    }
                    Err(e) => {
                        tracing::warn!("Failed to read from {remote}: {e}");
                    }
                }
            });
        }

        Ok(())
    }
}

/// Spawn a background task that monitors path changes for a connection.
fn spawn_path_watcher(
    connection: &Connection,
    remote: NodeId,
    tx: broadcast::Sender<PathEvent>,
) {
    let paths = connection.paths();
    let mut stream = paths.stream();
    let mut last_kind = PathKind::Unknown;

    tokio::spawn(async move {
        while let Some(path_info) = stream.next().await {
            let (kind, rtt) = classify_path(&path_info);

            if kind != last_kind {
                last_kind = kind;
                let event = PathEvent {
                    kind,
                    rtt,
                    remote,
                    timestamp: Instant::now(),
                };
                // Ignore send errors (no subscribers)
                let _ = tx.send(event);
            }
        }
    });
}

/// Classify the current path from the PathInfoList.
fn classify_path(
    paths: &tom_connect::endpoint::PathInfoList,
) -> (PathKind, std::time::Duration) {
    for path in paths.iter() {
        if path.is_selected() {
            if path.is_relay() {
                return (PathKind::Relay, path.rtt());
            } else {
                return (PathKind::Direct, path.rtt());
            }
        }
    }
    (PathKind::Unknown, std::time::Duration::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node_id() -> NodeId {
        NodeId::from_endpoint_id(tom_base::SecretKey::generate(&mut rand::rng()).public())
    }

    /// Un seul chunk annonçant un `total_chunks` énorme ne doit RIEN allouer :
    /// la garde `> MAX_CHUNKS` le rejette avant toute construction de buffer.
    /// (Régression : `vec![None; total_chunks]` forçait ~1,5 Go depuis 1 paquet.)
    #[test]
    fn rejette_amplification_total_chunks() {
        let buffers: ChunkBuffers = Default::default();
        let remote = node_id();

        // total_chunks = u32::MAX, payload 1 octet → doit être ignoré.
        let frame = encode_chunk(1, u32::MAX, 0, &[0u8]);
        assert!(reassemble(&buffers, remote, frame).is_none());
        assert!(
            buffers.lock().unwrap().is_empty(),
            "aucun buffer ne doit être alloué pour un total_chunks abusif"
        );

        // Juste au-dessus du plafond légitime → rejeté aussi.
        let frame = encode_chunk(2, MAX_CHUNKS + 1, 0, &[0u8]);
        assert!(reassemble(&buffers, remote, frame).is_none());
        assert!(buffers.lock().unwrap().is_empty());
    }

    /// Un nouveau transfert est refusé si le budget mémoire global est déjà plein
    /// (protège les appareils contraints d'un flot de gros fichiers concurrents).
    #[test]
    fn respecte_budget_global() {
        let buffers: ChunkBuffers = Default::default();
        let remote = node_id();

        // Pré-remplir le budget sans allouer réellement 128 Mo : on injecte une
        // entrée dont le compteur `bytes` sature le plafond.
        buffers.lock().unwrap().insert(
            (remote, 1),
            Reassembly {
                total_chunks: 2,
                chunks: std::collections::BTreeMap::new(),
                bytes: MAX_TOTAL_REASSEMBLY,
                last_activity: std::time::Instant::now(),
            },
        );

        // Un chunk d'un NOUVEAU transfert doit être rejeté (budget saturé).
        let frame = encode_chunk(2, 4, 0, &[7u8; 1000]);
        assert!(reassemble(&buffers, remote, frame).is_none());
        assert!(
            !buffers.lock().unwrap().contains_key(&(remote, 2)),
            "le transfert refusé ne doit pas créer d'entrée"
        );
    }

    /// Réassemblage correct même avec des chunks reçus dans le désordre :
    /// la `BTreeMap` garantit la concaténation en ordre d'index.
    #[test]
    fn reassemble_dans_le_desordre() {
        let buffers: ChunkBuffers = Default::default();
        let remote = node_id();
        let tid = 42;

        // 3 chunks distincts, envoyés dans l'ordre 2, 0, 1.
        let c0 = b"AAAA".to_vec();
        let c1 = b"BBBB".to_vec();
        let c2 = b"CCCC".to_vec();

        assert!(reassemble(&buffers, remote, encode_chunk(tid, 3, 2, &c2)).is_none());
        assert!(reassemble(&buffers, remote, encode_chunk(tid, 3, 0, &c0)).is_none());
        let full = reassemble(&buffers, remote, encode_chunk(tid, 3, 1, &c1))
            .expect("le message doit être complet au 3ᵉ chunk");

        assert_eq!(full, b"AAAABBBBCCCC", "concaténation dans l'ordre des index");
        assert!(
            buffers.lock().unwrap().is_empty(),
            "le buffer doit être purgé après réassemblage complet"
        );
    }

    /// Un chunk en double (même index) ne compte pas deux fois et ne fait pas
    /// croître `bytes` ni le compteur de complétude.
    #[test]
    fn chunk_duplique_ignore() {
        let buffers: ChunkBuffers = Default::default();
        let remote = node_id();
        let tid = 7;

        assert!(reassemble(&buffers, remote, encode_chunk(tid, 2, 0, b"XXXX")).is_none());
        // Renvoi du même index 0 → ignoré, transfert toujours incomplet.
        assert!(reassemble(&buffers, remote, encode_chunk(tid, 2, 0, b"XXXX")).is_none());
        let full = reassemble(&buffers, remote, encode_chunk(tid, 2, 1, b"YYYY"))
            .expect("complet après l'index 1");
        assert_eq!(full, b"XXXXYYYY");
    }

    /// Anti-spam : un pair ne peut pas ouvrir plus de MAX_CONCURRENT_PER_PEER
    /// transferts partiels simultanés (chacun sous le plafond mais nombreux).
    #[test]
    fn rejette_trop_de_transferts_concurrents_par_pair() {
        let buffers: ChunkBuffers = Default::default();
        let remote = node_id();

        // Ouvrir MAX transferts partiels (chunk 0 d'un total de 2 → reste incomplet).
        for tid in 0..MAX_CONCURRENT_PER_PEER as u64 {
            assert!(reassemble(&buffers, remote, encode_chunk(tid, 2, 0, b"aaaa")).is_none());
        }
        assert_eq!(buffers.lock().unwrap().len(), MAX_CONCURRENT_PER_PEER);

        // Le transfert suivant (nouveau transfer_id) doit être rejeté.
        let extra = MAX_CONCURRENT_PER_PEER as u64 + 1;
        assert!(reassemble(&buffers, remote, encode_chunk(extra, 2, 0, b"bbbb")).is_none());
        assert!(
            !buffers.lock().unwrap().contains_key(&(remote, extra)),
            "un transfert au-delà du cap par pair ne doit pas créer d'entrée"
        );
        // Mais un chunk d'un transfert DÉJÀ ouvert passe toujours.
        let full = reassemble(&buffers, remote, encode_chunk(0, 2, 1, b"cccc"))
            .expect("le transfert 0 doit pouvoir se compléter");
        assert_eq!(full, b"aaaacccc");
    }

    /// TTL : un transfert partiel inactif au-delà de REASSEMBLY_TTL est purgé
    /// lorsqu'un nouveau chunk arrive (zombie : 1er chunk puis jamais la suite).
    #[test]
    fn purge_les_transferts_zombies() {
        // Sur une machine tout juste démarrée on ne peut pas remonter le temps —
        // le test devient alors un no-op plutôt que de paniquer.
        let Some(old) = std::time::Instant::now()
            .checked_sub(REASSEMBLY_TTL + std::time::Duration::from_secs(5))
        else {
            return;
        };

        let buffers: ChunkBuffers = Default::default();
        let zombie = node_id();
        let fresh = node_id();

        // Injecter un zombie : partiel, dernière activité au-delà du TTL.
        buffers.lock().unwrap().insert(
            (zombie, 42),
            Reassembly {
                total_chunks: 4,
                chunks: std::collections::BTreeMap::new(),
                bytes: 1000,
                last_activity: old,
            },
        );

        // Un chunk d'un autre transfert déclenche la purge du zombie.
        reassemble(&buffers, fresh, encode_chunk(1, 2, 0, b"zzzz"));
        assert!(
            !buffers.lock().unwrap().contains_key(&(zombie, 42)),
            "le transfert zombie inactif doit avoir été purgé"
        );
    }
}
