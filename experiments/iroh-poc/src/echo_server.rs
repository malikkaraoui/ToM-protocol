//! ToM Protocol - iroh PoC: Echo Server
//!
//! This node binds an iroh endpoint, prints its EndpointId (public key),
//! and waits for incoming connections. Any data received on a bi-directional
//! QUIC stream is echoed back to the sender.
//!
//! Usage:
//!   cargo run --bin echo-server
//!
//! The printed EndpointId is needed by the connecting node.

use std::sync::Arc;

use anyhow::Result;
use iroh::protocol::{AcceptError, Router};
use iroh::Endpoint;
use tracing::info;

/// ALPN protocol identifier for our PoC
const TOM_ALPN: &[u8] = b"tom-protocol/poc/echo/0";

#[derive(Debug, Clone)]
struct EchoProtocol;

impl iroh::protocol::ProtocolHandler for EchoProtocol {
    async fn accept(
        &self,
        connection: iroh::endpoint::Connection,
    ) -> Result<(), AcceptError> {
        let remote = connection.remote_id();
        info!("accepted connection from {remote}");

        // Accept a bi-directional stream
        let (mut send, mut recv) = connection
            .accept_bi()
            .await
            .map_err(AcceptError::from_err)?;

        // Read all incoming data
        let data = recv
            .read_to_end(64 * 1024)
            .await
            .map_err(AcceptError::from_err)?;
        let msg = String::from_utf8_lossy(&data);
        info!("received: \"{msg}\" ({} bytes)", data.len());

        // Echo it back
        send.write_all(&data)
            .await
            .map_err(AcceptError::from_err)?;
        send.finish().map_err(AcceptError::from_err)?;
        info!("echoed back {} bytes", data.len());

        // Wait for the connection to close gracefully
        connection.closed().await;
        info!("connection closed");

        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    println!("=== ToM Protocol - iroh PoC: Echo Server ===\n");

    // Create an iroh endpoint with default discovery (DNS + Pkarr)
    let endpoint = Endpoint::bind().await?;

    // Print connection info
    let my_id = endpoint.id();
    println!("Endpoint ID (public key): {my_id}");

    let my_addr = endpoint.addr();
    println!("Endpoint address: {my_addr:?}");

    let local_addrs = endpoint.bound_sockets();
    println!("Local sockets: {local_addrs:?}");
    println!();

    // Build and start the protocol router
    let router = Router::builder(endpoint.clone())
        .accept(TOM_ALPN.to_vec(), Arc::new(EchoProtocol))
        .spawn();

    println!("Echo server listening...");
    println!("Waiting for connections. Press Ctrl+C to stop.\n");

    // Wait until Ctrl+C
    tokio::signal::ctrl_c().await?;

    println!("\nShutting down...");
    router.shutdown().await?;

    Ok(())
}
