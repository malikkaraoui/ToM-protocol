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

/// Buffer de réassemblage d'un transfert segmenté.
pub(crate) struct Reassembly {
    total_chunks: u32,
    chunks: Vec<Option<Vec<u8>>>,
    received: u32,
    bytes: usize,
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
fn reassemble(state: &HandlerState, remote: NodeId, data: Vec<u8>) -> Option<Vec<u8>> {
    // Frame normale (pas un chunk) → message complet directement.
    if data.len() < CHUNK_HEADER || &data[0..4] != CHUNK_MAGIC {
        return Some(data);
    }
    let transfer_id = u64::from_be_bytes(data[4..12].try_into().ok()?);
    let total_chunks = u32::from_be_bytes(data[12..16].try_into().ok()?);
    let index = u32::from_be_bytes(data[16..20].try_into().ok()?);
    let payload = data[CHUNK_HEADER..].to_vec();

    if total_chunks == 0
        || index >= total_chunks
        || total_chunks as usize > MAX_REASSEMBLED / 1.max(payload.len())
    {
        tracing::warn!("chunk invalide de {remote} (total={total_chunks}, idx={index}) — ignoré");
        return None;
    }

    let mut buffers = state.chunk_buffers.lock().unwrap_or_else(|e| e.into_inner());
    let entry = buffers.entry((remote, transfer_id)).or_insert_with(|| Reassembly {
        total_chunks,
        chunks: vec![None; total_chunks as usize],
        received: 0,
        bytes: 0,
    });

    if let Some(slot) = entry.chunks.get_mut(index as usize) {
        if slot.is_none() {
            entry.bytes += payload.len();
            entry.received += 1;
            *slot = Some(payload);
        }
    }

    if entry.bytes > MAX_REASSEMBLED {
        tracing::warn!("transfert {transfer_id} de {remote} dépasse {MAX_REASSEMBLED}o — abandon");
        buffers.remove(&(remote, transfer_id));
        return None;
    }

    if entry.received == entry.total_chunks {
        let entry = buffers.remove(&(remote, transfer_id))?;
        let mut full = Vec::with_capacity(entry.bytes);
        for chunk in entry.chunks.into_iter().flatten() {
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
                        if let Some(full) = reassemble(&state, remote, data) {
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
