//! ToM Protocol - iroh PoC: Chat Node
//!
//! Combines gossip peer discovery + direct QUIC messaging.
//! This is the closest approximation to how ToM Protocol will work:
//!   1. Nodes discover each other via gossip (HyParView/PlumTree)
//!   2. Messages are sent as direct QUIC streams (not through gossip)
//!   3. Gossip is for discovery/signaling, QUIC is for payload
//!
//! Usage:
//!   # First node:
//!   cargo run --bin chat-node -- --name Alice
//!
//!   # Second node (bootstrap from first):
//!   cargo run --bin chat-node -- --name Bob --peer <ALICE_ID>
//!
//!   # Then type messages in either terminal to send to all discovered peers.

use std::collections::HashMap;
use std::io::Write;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use iroh::protocol::{AcceptError, Router};
use iroh::{Endpoint, EndpointAddr, EndpointId};
use iroh_gossip::api::Event;
use iroh_gossip::{Gossip, TopicId};
use n0_future::StreamExt;
use tokio::sync::Mutex;

/// ALPN for direct chat messages
const CHAT_ALPN: &[u8] = b"tom-protocol/poc/chat/0";

/// Fixed topic for peer discovery
const DISCOVERY_TOPIC: [u8; 32] = *b"tom-protocol-poc-discovery-top01";

#[derive(Parser, Debug)]
#[command(name = "chat-node", about = "ToM chat PoC: gossip discovery + direct QUIC")]
struct Args {
    /// Display name for this node
    #[arg(short, long, default_value = "Anonymous")]
    name: String,

    /// EndpointId of a bootstrap peer
    #[arg(short, long)]
    peer: Option<String>,
}

/// Known peers discovered via gossip
type PeerMap = Arc<Mutex<HashMap<EndpointId, String>>>;

/// Protocol handler for incoming direct chat messages
#[derive(Debug, Clone)]
struct ChatHandler {
    my_name: String,
    start: Instant,
}

impl iroh::protocol::ProtocolHandler for ChatHandler {
    async fn accept(
        &self,
        connection: iroh::endpoint::Connection,
    ) -> Result<(), AcceptError> {
        let (mut send, mut recv) = connection
            .accept_bi()
            .await
            .map_err(AcceptError::from_err)?;

        let data = recv
            .read_to_end(64 * 1024)
            .await
            .map_err(AcceptError::from_err)?;

        let msg = String::from_utf8_lossy(&data);
        let elapsed = self.start.elapsed();

        // Parse "name|message" format
        let (sender_name, content) = msg
            .split_once('|')
            .unwrap_or(("?", &msg));

        println!(
            "\r[{elapsed:.1?}] {} > {content}",
            sender_name,
        );
        print!("[{}] > ", self.my_name);
        std::io::stdout().flush().ok();

        // Signal sender we're done by closing our send stream
        send.finish().map_err(AcceptError::from_err)?;

        Ok(())
    }
}

/// Send a direct QUIC message to a peer
async fn send_direct_message(
    endpoint: &Endpoint,
    target: EndpointId,
    my_name: &str,
    message: &str,
) -> Result<()> {
    let addr = EndpointAddr::new(target);
    let conn = endpoint.connect(addr, CHAT_ALPN).await?;

    let (mut send, mut recv) = conn.open_bi().await?;
    let payload = format!("{my_name}|{message}");
    send.write_all(payload.as_bytes()).await?;
    send.finish()?;

    // Wait for the receiver to process and close the connection
    // (read_to_end will complete when the peer closes the stream or connection)
    let _ = recv.read_to_end(0).await;

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let start = Instant::now();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn".into()),
        )
        .init();

    println!("=== ToM Protocol - Chat PoC ({}) ===\n", args.name);

    // Create iroh endpoint
    let endpoint = Endpoint::bind().await?;
    let my_id = endpoint.id();
    println!("Your ID: {my_id}");

    // Build protocols
    let gossip = Gossip::builder().spawn(endpoint.clone());
    let chat_handler = ChatHandler {
        my_name: args.name.clone(),
        start,
    };

    let _router = Router::builder(endpoint.clone())
        .accept(iroh_gossip::ALPN.to_vec(), gossip.clone())
        .accept(CHAT_ALPN.to_vec(), Arc::new(chat_handler))
        .spawn();

    // Bootstrap peers
    let bootstrap_peers: Vec<EndpointId> = if let Some(ref peer_str) = args.peer {
        let peer_id: EndpointId = peer_str.parse().context("Invalid peer ID")?;
        println!("Bootstrap: {}", &peer_id.to_string()[..10]);
        vec![peer_id]
    } else {
        println!("No bootstrap - share your ID with others.");
        vec![]
    };

    // Join discovery topic
    let topic_id = TopicId::from_bytes(DISCOVERY_TOPIC);
    let (gossip_sender, mut gossip_receiver) = gossip
        .subscribe(topic_id, bootstrap_peers)
        .await?
        .split();

    // Track discovered peers
    let peers: PeerMap = Arc::new(Mutex::new(HashMap::new()));

    // Announce ourselves via gossip
    let announce = format!("ANNOUNCE|{}", args.name);
    gossip_sender
        .broadcast(announce.into_bytes().into())
        .await
        .ok();

    // Spawn gossip listener (discovers peers)
    let peers_clone = peers.clone();
    let name_clone = args.name.clone();
    let gossip_sender_clone = gossip_sender.clone();
    tokio::spawn(async move {
        while let Some(event) = gossip_receiver.next().await {
            match event {
                Ok(Event::Received(msg)) => {
                    let content = String::from_utf8_lossy(&msg.content);
                    if let Some(peer_name) = content.strip_prefix("ANNOUNCE|") {
                        let peer_id = msg.delivered_from;
                        let mut map = peers_clone.lock().await;
                        if !map.contains_key(&peer_id) {
                            println!(
                                "\r[DISCOVERED] {} ({})",
                                peer_name,
                                &peer_id.to_string()[..10]
                            );
                            map.insert(peer_id, peer_name.to_string());
                            print!("[{name_clone}] > ");
                            std::io::stdout().flush().ok();

                            // Announce back so they know us too
                            let re_announce = format!("ANNOUNCE|{name_clone}");
                            gossip_sender_clone
                                .broadcast(re_announce.into_bytes().into())
                                .await
                                .ok();
                        }
                    }
                }
                Ok(Event::NeighborUp(peer)) => {
                    println!(
                        "\r[NEIGHBOR UP] {}",
                        &peer.to_string()[..10]
                    );
                    print!("[{name_clone}] > ");
                    std::io::stdout().flush().ok();

                    // Re-announce when a new neighbor connects
                    // (initial announce may have been sent before any neighbors existed)
                    let re_announce = format!("ANNOUNCE|{name_clone}");
                    gossip_sender_clone
                        .broadcast(re_announce.into_bytes().into())
                        .await
                        .ok();
                }
                Ok(Event::NeighborDown(peer)) => {
                    let mut map = peers_clone.lock().await;
                    map.remove(&peer);
                    println!(
                        "\r[NEIGHBOR DOWN] {}",
                        &peer.to_string()[..10]
                    );
                    print!("[{name_clone}] > ");
                    std::io::stdout().flush().ok();
                }
                _ => {}
            }
        }
    });

    // Interactive chat loop
    println!("\nType a message and press Enter to send to all peers.");
    println!("Type /peers to list discovered peers. Ctrl+C to quit.\n");

    let mut line = String::new();

    loop {
        print!("[{}] > ", args.name);
        std::io::stdout().flush()?;

        line.clear();
        let read_result = tokio::task::spawn_blocking({
            let mut line = line.clone();
            move || {
                let n = std::io::stdin().read_line(&mut line)?;
                Ok::<(usize, String), std::io::Error>((n, line))
            }
        })
        .await??;

        let (n, bytes) = read_result;
        // EOF: pipe closed or Ctrl+D
        if n == 0 {
            println!("\n[{}] EOF - shutting down.", args.name);
            break;
        }

        let input = bytes.trim();
        if input.is_empty() {
            continue;
        }

        if input == "/peers" {
            let map = peers.lock().await;
            if map.is_empty() {
                println!("  No peers discovered yet.");
            } else {
                println!("  Discovered peers:");
                for (id, name) in map.iter() {
                    println!("    {} ({})", name, &id.to_string()[..10]);
                }
            }
            continue;
        }

        // Send to all discovered peers via direct QUIC
        let map = peers.lock().await;
        if map.is_empty() {
            println!("  No peers yet - waiting for gossip discovery...");
            continue;
        }

        let targets: Vec<(EndpointId, String)> = map
            .iter()
            .map(|(id, name)| (*id, name.clone()))
            .collect();
        drop(map);

        for (peer_id, peer_name) in &targets {
            match send_direct_message(&endpoint, *peer_id, &args.name, input).await {
                Ok(()) => {}
                Err(e) => {
                    println!("  [ERROR] Failed to send to {peer_name}: {e}");
                }
            }
        }
    }

    Ok(())
}
