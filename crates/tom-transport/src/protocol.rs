use crate::envelope::MessageEnvelope;
use crate::path::{PathEvent, PathKind};
use crate::{NodeId, TomTransportError};

use iroh::endpoint::Connection;
use iroh::protocol::AcceptError;
use n0_future::StreamExt;
use n0_watcher::Watcher;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{broadcast, mpsc};

/// Write a length-prefixed message to a QUIC send stream.
pub(crate) async fn write_framed(
    send: &mut iroh::endpoint::SendStream,
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
    recv: &mut iroh::endpoint::RecvStream,
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

/// Internal state shared with the protocol handler.
pub(crate) struct HandlerState {
    pub incoming_tx: mpsc::Sender<(NodeId, MessageEnvelope)>,
    pub incoming_raw_tx: mpsc::Sender<(NodeId, Vec<u8>)>,
    pub path_event_tx: broadcast::Sender<PathEvent>,
    pub max_message_size: usize,
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

impl iroh::protocol::ProtocolHandler for TomProtocolHandler {
    async fn accept(&self, connection: Connection) -> Result<(), AcceptError> {
        let remote = NodeId::from_endpoint_id(connection.remote_id());
        let state = self.state.clone();

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
                        // Try to parse as envelope
                        match MessageEnvelope::from_bytes(&data) {
                            Ok(envelope) => {
                                let _ = state.incoming_tx.send((remote, envelope)).await;
                            }
                            Err(_) => {
                                // Not a valid envelope — deliver as raw
                                let _ = state.incoming_raw_tx.send((remote, data)).await;
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

/// Classify the current path from iroh's PathInfoList.
fn classify_path(
    paths: &iroh::endpoint::PathInfoList,
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
