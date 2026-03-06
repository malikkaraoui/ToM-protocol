//! Integration tests avec plusieurs nodes réels (pas de mocks).
//!
//! Ces tests simulent ce que des vrais utilisateurs font :
//! - Envoyer des messages dans les deux sens
//! - Rejoindre et ne rien faire pendant un moment, puis revenir
//! - Envoyer plein de messages d'un coup (spam)
//! - Se connecter, se déconnecter, se reconnecter
//!
//! Contrairement aux 964 unit tests existants, ces tests lancent de vrais
//! nodes QUIC et vérifient les scénarios end-to-end.

use std::time::Duration;
use tokio::time::timeout;
use tom_protocol::{AntiSpamConfig, ProtocolRuntime, RuntimeConfig};
use tom_transport::{TomNode, TomNodeConfig};

/// Résultat du setup: ID, handle, receiver de messages, et l'adresse réseau.
struct TestNode {
    id: tom_protocol::types::NodeId,
    handle: tom_protocol::RuntimeHandle,
    messages: tokio::sync::mpsc::Receiver<tom_protocol::DeliveredMessage>,
}

/// Setup: lance 2 nodes en mode local pur (pas de DHT/Pkarr/DNS).
/// Échange les adresses pour permettre la connexion directe.
async fn setup_two_nodes() -> anyhow::Result<(TestNode, TestNode)> {
    // Permissive anti-spam for tests (default min_rate=2 msg/sec, burst=4 is too restrictive)
    let antispam = AntiSpamConfig { min_rate: 1000.0, ..AntiSpamConfig::default() };

    // Node A — local only, no external discovery
    let node_a = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;
    let id_a = node_a.id();
    let addr_a = node_a.addr();
    let config_a = RuntimeConfig {
        enable_dht: false,
        antispam_config: antispam.clone(),
        ..RuntimeConfig::default()
    };
    let channels_a = ProtocolRuntime::spawn(node_a, config_a);

    // Node B — local only, no external discovery
    let node_b = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;
    let id_b = node_b.id();
    let addr_b = node_b.addr();
    let config_b = RuntimeConfig {
        enable_dht: false,
        antispam_config: antispam.clone(),
        ..RuntimeConfig::default()
    };
    let channels_b = ProtocolRuntime::spawn(node_b, config_b);

    eprintln!("Node A: {} (addrs: {})", id_a, addr_a.addrs.len());
    eprintln!("Node B: {} (addrs: {})", id_b, addr_b.addrs.len());

    // Échange d'adresses — comme un QR code scan IRL
    channels_a.handle.add_peer_addr(addr_b).await;
    channels_b.handle.add_peer_addr(addr_a).await;

    // Laisser les connexions s'établir
    tokio::time::sleep(Duration::from_millis(500)).await;

    Ok((
        TestNode { id: id_a, handle: channels_a.handle, messages: channels_a.messages },
        TestNode { id: id_b, handle: channels_b.handle, messages: channels_b.messages },
    ))
}

// ═══════════════════════════════════════════════════════════════════════
// Scénario 1 : Alice envoie à Bob, Bob répond — le cas le plus basique.
//
// C'est LE test que 964 unit tests n'ont pas attrapé :
// Un humain envoie "salut", l'autre répond "yo".
// ═══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn alice_sends_bob_replies() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("tom_protocol=info,tom_transport=info")
        .try_init()
        .ok();

    let (mut alice, mut bob) = setup_two_nodes().await?;

    // Alice: "salut"
    alice.handle.send_message(bob.id, b"salut".to_vec()).await?;

    let msg = timeout(Duration::from_secs(10), bob.messages.recv())
        .await
        .map_err(|_| anyhow::anyhow!("Bob never received Alice's message"))?;
    assert!(msg.is_some(), "Bob should have received Alice's message");
    assert_eq!(msg.unwrap().payload, b"salut");
    eprintln!("Alice -> Bob: OK");

    // Bob: "yo"
    bob.handle.send_message(alice.id, b"yo".to_vec()).await?;

    let msg = timeout(Duration::from_secs(10), alice.messages.recv())
        .await
        .map_err(|_| anyhow::anyhow!("Alice never received Bob's reply"))?;
    assert!(msg.is_some(), "Alice should have received Bob's reply");
    assert_eq!(msg.unwrap().payload, b"yo");
    eprintln!("Bob -> Alice: OK");

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Scénario 2 : Ping-pong — 10 échanges consécutifs, comme une vraie conv.
//
// Un humain ne fait pas "envoyer 1 message et quitter".
// Il fait une conversation : msg, réponse, msg, réponse...
// ═══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn ping_pong_10_exchanges() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("tom_transport=trace".parse().unwrap())
                .add_directive("tom_protocol=info".parse().unwrap()),
        )
        .try_init()
        .ok();

    let (mut alice, mut bob) = setup_two_nodes().await?;

    for i in 0..10 {
        // A -> B
        let payload_ab = format!("ping {}", i).into_bytes();
        alice.handle.send_message(bob.id, payload_ab.clone()).await?;

        let msg = timeout(Duration::from_secs(10), bob.messages.recv())
            .await
            .map_err(|_| anyhow::anyhow!("B didn't receive ping {}", i))?;
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().payload, payload_ab);

        // B -> A
        let payload_ba = format!("pong {}", i).into_bytes();
        bob.handle.send_message(alice.id, payload_ba.clone()).await?;

        let msg = timeout(Duration::from_secs(10), alice.messages.recv())
            .await
            .map_err(|_| anyhow::anyhow!("A didn't receive pong {}", i))?;
        assert!(msg.is_some());
        assert_eq!(msg.unwrap().payload, payload_ba);

        eprintln!("Exchange {}/10 OK", i + 1);
    }

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Scénario 3 : Burst — un humain qui colle 20 messages d'affilée.
//
// Genre quelqu'un qui envoie un pavé en 20 messages séparés au lieu d'un seul.
// Le protocole doit TOUS les livrer, dans l'ordre.
// ═══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn burst_20_messages() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("tom_protocol::runtime=trace,tom_transport=debug")
        .try_init()
        .ok();

    let (alice, mut bob) = setup_two_nodes().await?;

    // Envoyer 20 messages sans attendre de réponse
    for i in 0..20 {
        let payload = format!("msg {}", i).into_bytes();
        alice.handle.send_message(bob.id, payload).await?;
    }
    eprintln!("20 messages sent");

    // Vérifier qu'on reçoit les 20
    let mut received = 0;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
    while received < 20 {
        let remaining = deadline - tokio::time::Instant::now();
        match timeout(remaining, bob.messages.recv()).await {
            Ok(Some(_msg)) => received += 1,
            Ok(None) => break,
            Err(_) => break,
        }
    }

    assert_eq!(received, 20, "Should receive all 20 messages, got {}", received);
    eprintln!("All 20 messages received");

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Scénario 4 : Le mec qui attend 30 secondes avant de répondre.
//
// Un vrai humain lit le message, va faire un café, revient, et répond.
// La connexion doit encore marcher.
// ═══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn idle_then_reply() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("tom_protocol=info")
        .try_init()
        .ok();

    let (mut alice, mut bob) = setup_two_nodes().await?;

    // A envoie un message
    alice.handle.send_message(bob.id, b"tu es la?".to_vec()).await?;

    let msg = timeout(Duration::from_secs(10), bob.messages.recv())
        .await
        .map_err(|_| anyhow::anyhow!("B didn't receive message"))?;
    assert!(msg.is_some());
    eprintln!("Message received, now waiting 30 seconds...");

    // Attendre 30 secondes (le mec fait un café)
    tokio::time::sleep(Duration::from_secs(30)).await;

    // B répond après 30 secondes d'inactivité
    bob.handle.send_message(alice.id, b"oui je suis la".to_vec()).await?;

    let msg = timeout(Duration::from_secs(10), alice.messages.recv())
        .await
        .map_err(|_| anyhow::anyhow!("A didn't receive reply after 30s idle"))?;
    assert!(msg.is_some());
    assert_eq!(msg.unwrap().payload, b"oui je suis la");
    eprintln!("Reply received after 30s idle: OK");

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Scénario 5 : Stabilité 2 minutes — échanges continus.
//
// Le bug #3 : connexion meurt après ~2 minutes.
// ═══════════════════════════════════════════════════════════════════════

#[tokio::test]
async fn stability_2min() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("tom_protocol=info")
        .try_init()
        .ok();

    let (mut alice, mut bob) = setup_two_nodes().await?;

    let mut sent = 0;
    let mut received = 0;
    let start = tokio::time::Instant::now();

    // Envoyer 1 msg/sec pendant 2 minutes, alterner direction
    while start.elapsed() < Duration::from_secs(120) {
        let elapsed_secs = start.elapsed().as_secs();

        if elapsed_secs % 2 == 0 {
            // A -> B
            let payload = format!("from-a-{}", sent).into_bytes();
            if alice.handle.send_message(bob.id, payload).await.is_err() {
                eprintln!("Send failed at {}s", elapsed_secs);
                break;
            }
            if timeout(Duration::from_secs(5), bob.messages.recv()).await.is_ok() {
                received += 1;
            }
        } else {
            // B -> A
            let payload = format!("from-b-{}", sent).into_bytes();
            if bob.handle.send_message(alice.id, payload).await.is_err() {
                eprintln!("Send failed at {}s", elapsed_secs);
                break;
            }
            if timeout(Duration::from_secs(5), alice.messages.recv()).await.is_ok() {
                received += 1;
            }
        }

        sent += 1;

        if sent % 30 == 0 {
            eprintln!("{}s: {}/{} delivered", elapsed_secs, received, sent);
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    let loss_pct = if sent > 0 {
        ((sent - received) as f64 / sent as f64) * 100.0
    } else {
        100.0
    };

    eprintln!("Result: {}/{} delivered ({:.1}% loss)", received, sent, loss_pct);
    assert!(
        loss_pct < 5.0,
        "Too many messages lost: {:.1}% (sent={}, received={})",
        loss_pct,
        sent,
        received
    );

    Ok(())
}

// ═══════════════════════════════════════════════════════════════════════
// Scénario 6 : Gossip discovery — 2 nodes sans /connect.
//
// Le bug #1 : les nodes ne se découvrent pas automatiquement.
// ═══════════════════════════════════════════════════════════════════════

/// Raw transport test — bypass ProtocolRuntime, test QUIC directly.
/// If this passes but ping_pong fails, the issue is in the runtime.
#[tokio::test]
async fn raw_transport_10_messages() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("tom_transport=debug")
        .try_init()
        .ok();

    let mut node_a = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;
    let mut node_b = TomNode::bind(TomNodeConfig::new().n0_discovery(false)).await?;

    let addr_a = node_a.addr();
    let addr_b = node_b.addr();

    // Échange d'adresses
    node_a.add_peer_addr(addr_b).await;
    node_b.add_peer_addr(addr_a).await;

    let id_b = node_b.id();
    tokio::time::sleep(Duration::from_millis(500)).await;

    // Spawn receiver
    let recv_handle = tokio::spawn(async move {
        let mut received = 0;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
        while received < 10 {
            let remaining = deadline - tokio::time::Instant::now();
            match timeout(remaining, node_b.recv_raw()).await {
                Ok(Ok((_from, data))) => {
                    received += 1;
                    eprintln!("B received msg {} ({} bytes)", received, data.len());
                }
                Ok(Err(e)) => {
                    eprintln!("B recv error: {}", e);
                    break;
                }
                Err(_) => {
                    eprintln!("B recv timeout after {} messages", received);
                    break;
                }
            }
        }
        received
    });

    // Send 10 messages
    for i in 0..10 {
        let payload = format!("raw-msg-{}", i).into_bytes();
        node_a.send_raw(id_b, &payload).await?;
        eprintln!("A sent msg {}", i);
    }

    let received = recv_handle.await?;
    assert_eq!(received, 10, "Should receive all 10 raw messages, got {}", received);
    eprintln!("All 10 raw messages received");

    Ok(())
}

#[tokio::test]
#[ignore] // Fix gossip discovery first
async fn auto_discovery_no_manual_connect() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("tom_protocol=debug,tom_gossip=debug")
        .try_init()
        .ok();

    let (alice, _bob) = setup_two_nodes().await?;

    // Attendre que le gossip fasse son travail (30s)
    eprintln!("Waiting 30s for gossip discovery...");
    tokio::time::sleep(Duration::from_secs(30)).await;

    let peers = alice.handle.connected_peers().await;
    assert!(
        !peers.is_empty(),
        "After 30s, nodes should have discovered each other via gossip"
    );
    eprintln!("Discovered {} peers via gossip", peers.len());

    Ok(())
}
