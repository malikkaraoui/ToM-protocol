//! ToM Protocol - iroh PoC: Connecting Node
//!
//! This node connects to an echo-server by its EndpointId,
//! sends a message, and reads back the echo.
//!
//! Usage:
//!   cargo run --bin node -- <ENDPOINT_ID> [MESSAGE]
//!
//! Example:
//!   cargo run --bin node -- 3f5a...b2c1 "Hello from ToM Protocol!"

use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use iroh::{Endpoint, EndpointAddr, EndpointId};

/// ALPN protocol identifier - must match echo-server
const TOM_ALPN: &[u8] = b"tom-protocol/poc/echo/0";

#[derive(Parser, Debug)]
#[command(name = "tom-node", about = "Connect to a ToM iroh echo server")]
struct Args {
    /// EndpointId (public key) of the echo-server to connect to
    endpoint_id: String,

    /// Message to send (default: "Hello from ToM Protocol!")
    #[arg(default_value = "Hello from ToM Protocol!")]
    message: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    println!("=== ToM Protocol - iroh PoC: Connecting Node ===\n");

    // Parse the target endpoint ID
    let target_id: EndpointId = args
        .endpoint_id
        .parse()
        .context("Invalid EndpointId format")?;

    // Create EndpointAddr from just the ID - iroh discovery will resolve the rest
    let target_addr = EndpointAddr::new(target_id);

    println!("Target: {target_id}");
    println!("Message: \"{}\"", args.message);
    println!();

    // Create our own endpoint with default discovery (DNS + Pkarr)
    let endpoint = Endpoint::bind().await?;

    let my_id = endpoint.id();
    println!("Our Endpoint ID: {my_id}");

    let my_addr = endpoint.addr();
    println!("Our address: {my_addr:?}");
    println!();

    // Connect to the echo server via discovery
    println!("Connecting to {target_id}...");
    let connect_start = Instant::now();

    let connection = endpoint
        .connect(target_addr, TOM_ALPN)
        .await
        .context("Failed to connect to echo server")?;

    let connect_time = connect_start.elapsed();
    let remote = connection.remote_id();
    println!("Connected to {remote} in {connect_time:?}");
    println!();

    // Open a bi-directional stream and send our message
    println!("Sending message...");
    let send_start = Instant::now();

    let (mut send, mut recv) = connection.open_bi().await?;
    send.write_all(args.message.as_bytes()).await?;
    send.finish()?;

    // Read back the echo
    let response = recv.read_to_end(64 * 1024).await?;
    let round_trip = send_start.elapsed();

    let echo = String::from_utf8_lossy(&response);
    println!("Echo received: \"{echo}\"");
    println!("Round-trip time: {round_trip:?}");
    println!();

    // Verify
    let verified = response == args.message.as_bytes();
    if verified {
        println!("SUCCESS: Echo matches sent message!");
    } else {
        println!("MISMATCH: Echo does not match!");
    }

    // Print summary
    println!("\n=== PoC Results ===");
    println!("Connection time: {connect_time:?}");
    println!("Round-trip time: {round_trip:?}");
    println!("Message size: {} bytes", args.message.len());
    println!("Echo verified: {verified}");

    // Close gracefully
    connection.close(0u32.into(), b"done");
    endpoint.close().await;

    Ok(())
}
