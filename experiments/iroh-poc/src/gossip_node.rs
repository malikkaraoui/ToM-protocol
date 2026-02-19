//! ToM Protocol - iroh PoC: Gossip Node
//!
//! Multiple instances join a shared topic via iroh-gossip (HyParView/PlumTree).
//! Each node broadcasts messages and receives messages from all other peers.
//!
//! Usage:
//!   # First node (creates the topic, no bootstrap peer needed):
//!   cargo run --bin gossip-node -- --name Alice
//!
//!   # Second node (bootstraps from first node's EndpointId):
//!   cargo run --bin gossip-node -- --name Bob --peer <ALICE_ENDPOINT_ID>
//!
//!   # Third node (bootstraps from any existing node):
//!   cargo run --bin gossip-node -- --name Charlie --peer <BOB_ENDPOINT_ID>

use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use iroh::protocol::Router;
use iroh::{Endpoint, EndpointId};
use iroh_gossip::api::Event;
use iroh_gossip::{Gossip, TopicId};
use n0_future::StreamExt;
/// Fixed topic ID for the PoC (all nodes share this)
/// In production ToM, topics would be dynamic (per group, per subnet)
const TOM_TOPIC: [u8; 32] = *b"tom-protocol-poc-gossip-topic-01";

#[derive(Parser, Debug)]
#[command(name = "gossip-node", about = "ToM gossip discovery PoC")]
struct Args {
    /// Display name for this node
    #[arg(short, long, default_value = "Anonymous")]
    name: String,

    /// EndpointId of a peer already in the gossip network (bootstrap)
    #[arg(short, long)]
    peer: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    println!("=== ToM Protocol - iroh PoC: Gossip Node ({}) ===\n", args.name);

    // Create iroh endpoint
    let endpoint = Endpoint::bind().await?;
    let my_id = endpoint.id();
    println!("Endpoint ID: {my_id}");
    println!();

    // Build gossip protocol
    let gossip = Gossip::builder().spawn(endpoint.clone());

    // Setup router with gossip protocol handler
    let router = Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN.to_vec(), gossip.clone())
        .spawn();

    // Parse bootstrap peers
    let bootstrap_peers: Vec<EndpointId> = if let Some(ref peer_str) = args.peer {
        let peer_id: EndpointId = peer_str
            .parse()
            .context("Invalid peer EndpointId")?;
        println!("Bootstrap peer: {peer_id}");
        vec![peer_id]
    } else {
        println!("No bootstrap peer - this node starts a new gossip network.");
        println!("Other nodes should use --peer {my_id}");
        vec![]
    };
    println!();

    // Subscribe to our shared topic
    let topic_id = TopicId::from_bytes(TOM_TOPIC);
    println!("Joining topic: {topic_id}");

    let start = Instant::now();
    let (sender, mut receiver) = gossip
        .subscribe(topic_id, bootstrap_peers)
        .await?
        .split();

    // Spawn a task to read incoming gossip messages
    let node_name = args.name.clone();
    let recv_task = tokio::spawn(async move {
        println!("[{node_name}] Listening for gossip messages...\n");

        while let Some(event) = receiver.next().await {
            match event {
                Ok(Event::Received(msg)) => {
                    let content = String::from_utf8_lossy(&msg.content);
                    let from = msg.delivered_from;
                    let elapsed = start.elapsed();
                    println!(
                        "[{elapsed:.1?}] Message from {}: \"{}\"",
                        &from.to_string()[..10],
                        content,
                    );
                }
                Ok(Event::NeighborUp(peer)) => {
                    let elapsed = start.elapsed();
                    println!("[{elapsed:.1?}] Neighbor UP: {}", &peer.to_string()[..10]);
                }
                Ok(Event::NeighborDown(peer)) => {
                    let elapsed = start.elapsed();
                    println!("[{elapsed:.1?}] Neighbor DOWN: {}", &peer.to_string()[..10]);
                }
                Ok(Event::Lagged) => {
                    println!("[WARN] Receiver lagged, missed some events");
                }
                Err(e) => {
                    println!("[ERROR] Gossip error: {e}");
                    break;
                }
            }
        }
    });

    // Broadcast periodic messages
    println!("[{}] Sending messages every 5 seconds. Press Ctrl+C to stop.\n", args.name);

    let mut counter = 0u32;
    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                println!("\n[{}] Shutting down...", args.name);
                break;
            }
            _ = tokio::time::sleep(tokio::time::Duration::from_secs(5)) => {
                counter += 1;
                let msg = format!("[{}] message #{counter}", args.name);
                println!("[{}] Broadcasting: \"{msg}\"", args.name);

                if let Err(e) = sender.broadcast(msg.into_bytes().into()).await {
                    println!("[{}] Broadcast error: {e}", args.name);
                }
            }
        }
    }

    // Cleanup
    recv_task.abort();
    router.shutdown().await?;

    println!("[{}] Done.", args.name);
    Ok(())
}
