//! R15-lite — test déterministe exigé par `docs/plans/r15-annuaire-local.md` §5 :
//! un nœud redémarré retrouve un pair via le **relais habituel persisté**, sans
//! AUCUNE découverte (n0/mDNS/DHT off) ni relais configuré au restart.
//!
//! Chaîne prouvée : PathEvent RELAY (authentifié) → `relay_routes` (RuntimeState)
//! → colonne `preferred_relay_url` (save) → load M2 → semis du pool au démarrage
//! → dial via le relais → livraison.
//!
//! Hermétique : relais embarqué réel sur 127.0.0.1:0, aucun canal de découverte
//! actif, et surtout **tous les endpoints QUIC liés au loopback** (`127.0.0.1:0`).
//! Leçon du premier run : avec un bind wildcard, la vraie flotte du LAN a
//! découvert et AUTO-PINGÉ les nœuds de test (l'iPad a livré un message dans
//! `channels_b.messages` avant celui du test). Le loopback coupe la fuite dans
//! les deux sens ; l'assert filtre par expéditeur en défense résiduelle.

use std::time::Duration;
use tokio::time::timeout;
use tom_protocol::runtime::{EmbeddedRelayConfig, EmbeddedRelayService};
use tom_protocol::{AntiSpamConfig, ProtocolRuntime, RuntimeConfig};
use tom_transport::{EndpointAddr, TomNode, TomNodeConfig};

#[tokio::test]
async fn restarted_node_reconnects_via_cached_relay_route() -> anyhow::Result<()> {
    let antispam = AntiSpamConfig { min_rate: 1000.0, ..AntiSpamConfig::default() };
    let data_dir = tempfile::tempdir()?;

    // ── 1. Relais local réel ────────────────────────────────────────────
    let mut relay = EmbeddedRelayService::new();
    let relay_url = relay
        .start(EmbeddedRelayConfig {
            bind_addr: "127.0.0.1:0".parse()?,
            advertise_addr: Some("127.0.0.1".parse()?),
        })
        .await
        .map_err(|e| anyhow::anyhow!("relay start: {e}"))?;
    eprintln!("relais embarqué : {relay_url}");

    // ── 2. Node B — joignable via ce relais uniquement ──────────────────
    let node_b = TomNode::bind(
        TomNodeConfig::new()
            .n0_discovery(false)
            .local_discovery(false)
            .bind_addr("127.0.0.1:0".parse()?)
            .relay_url(relay_url.clone()),
    )
    .await?;
    let id_b = node_b.id();
    let endpoint_id_b = node_b.addr().id;
    let config_b = RuntimeConfig {
        enable_dht: false,
        antispam_config: antispam.clone(),
        ..RuntimeConfig::default()
    };
    let mut channels_b = ProtocolRuntime::spawn(node_b, config_b);

    // ── 3. Node A phase 1 — apprend la route relais de B, la persiste ───
    // Identité persistée (comme une vraie app) : mêmes clés au restart.
    let identity_a = data_dir.path().join("identity-a.key");
    let id_a = {
        let node_a = TomNode::bind(
            TomNodeConfig::new()
                .n0_discovery(false)
                .local_discovery(false)
                .bind_addr("127.0.0.1:0".parse()?)
                .identity_path(identity_a.clone()),
        )
        .await?;
        let id_a = node_a.id();
        let config_a = RuntimeConfig {
            enable_dht: false,
            antispam_config: antispam.clone(),
            data_dir: Some(data_dir.path().to_path_buf()),
            ..RuntimeConfig::default()
        };
        let mut channels_a = ProtocolRuntime::spawn(node_a, config_a);

        // A ne connaît B que par une adresse RELAIS-SEULE (pas d'adresse
        // directe) : le chemin sélectionné sera RELAY → PathEvent apprenable.
        let addr_b_relay_only =
            EndpointAddr::new(endpoint_id_b).with_relay_url(relay_url.clone());
        channels_a.handle.add_peer_addr(addr_b_relay_only).await;

        // Message A→B via le relais (établit la connexion + le PathEvent).
        channels_a
            .handle
            .send_message(id_b, b"phase1 via relais".to_vec())
            .await?;
        // Filtre par expéditeur : ne pas asserter sur le PREMIER message reçu
        // (défense résiduelle contre toute pollution externe).
        timeout(Duration::from_secs(20), async {
            loop {
                match channels_b.messages.recv().await {
                    Some(m) if m.from == id_a => break Ok(()),
                    Some(m) => eprintln!("phase 1 : message parasite de {} ignoré", m.from),
                    None => break Err(anyhow::anyhow!("canal B fermé")),
                }
            }
        })
        .await
        .map_err(|_| anyhow::anyhow!("B n'a pas reçu le message phase 1"))??;

        // Attendre le PathChanged RELAY côté A (la preuve que la route est
        // apprenable), puis flusher l'état — déterministe, pas de sleep.
        let learned = timeout(Duration::from_secs(20), async {
            while let Some(evt) = channels_a.events.recv().await {
                if let tom_protocol::ProtocolEvent::PathChanged { event } = evt {
                    if event.remote == id_b && event.addr.starts_with("relay:") {
                        return true;
                    }
                }
            }
            false
        })
        .await
        .unwrap_or(false);
        assert!(learned, "aucun PathEvent RELAY observé pour B en phase 1");

        channels_a.handle.save_now().await;
        channels_a.handle.shutdown().await;
        // Laisser le shutdown libérer le socket + fermer le store.
        tokio::time::sleep(Duration::from_millis(500)).await;
        id_a
    };

    // ── 4. Node A phase 2 — restart SANS relais configuré, SANS découverte.
    // Seule la route persistée (semée dans le pool au démarrage) peut
    // atteindre B.
    let node_a2 = TomNode::bind(
        TomNodeConfig::new()
            .n0_discovery(false)
            .local_discovery(false)
            .bind_addr("127.0.0.1:0".parse()?)
            .identity_path(identity_a),
    )
    .await?;
    assert_eq!(node_a2.id(), id_a, "identité conservée au restart");
    let config_a2 = RuntimeConfig {
        enable_dht: false,
        antispam_config: antispam,
        data_dir: Some(data_dir.path().to_path_buf()),
        ..RuntimeConfig::default()
    };
    let channels_a2 = ProtocolRuntime::spawn(node_a2, config_a2);

    // Aucun add_peer_addr ici : si ce send aboutit, c'est la route persistée.
    channels_a2
        .handle
        .send_message(id_b, b"phase2 depuis le cache".to_vec())
        .await?;
    timeout(Duration::from_secs(30), async {
        loop {
            match channels_b.messages.recv().await {
                Some(m) if m.from == id_a && m.payload == b"phase2 depuis le cache" => {
                    break Ok(())
                }
                Some(m) => eprintln!("phase 2 : message parasite de {} ignoré", m.from),
                None => break Err(anyhow::anyhow!("canal B fermé (phase 2)")),
            }
        }
    })
    .await
    .map_err(|_| {
        anyhow::anyhow!(
            "B n'a pas reçu le message phase 2 — la route relais persistée n'a pas été utilisée"
        )
    })??;

    channels_a2.handle.shutdown().await;
    channels_b.handle.shutdown().await;
    relay.stop().await;
    Ok(())
}
