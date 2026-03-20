# Design — Relay embedé (20 mars 2026)

## Objectif

Permettre à un node ToM de démarrer, superviser et arrêter un vrai `tom-relay` embarqué dans son propre process. Fondation du futur relay rotatif.

## Hors scope

- Publication automatique du relay au réseau
- Migration des autres nodes vers ce relay
- Rotation globale de relay
- Modification de `PeerAnnounce`

## Architecture

### Fichier principal

`crates/tom-protocol/src/runtime/embedded_relay.rs`

Le service vit côté orchestration async (boucle runtime), jamais dans `RuntimeState`.

### Types

```rust
/// Status du relay embedé — lifecycle local uniquement.
/// Healthy = serveur sain du point de vue lifecycle local,
/// PAS "publiable réseau".
pub enum EmbeddedRelayStatus {
    Stopped,
    Starting,
    Healthy,
    Failed(String),
}

/// Configuration MVP pour le relay embedé.
/// Extensible vers dev_mode, TLS, access policy.
pub struct EmbeddedRelayConfig {
    pub bind_addr: SocketAddr,
}

/// Service dédié encapsulant le lifecycle du relay embedé.
pub struct EmbeddedRelayService {
    status: EmbeddedRelayStatus,
    server: Option<tom_relay::server::Server>,
    config: Option<EmbeddedRelayConfig>,
    bound_relay_url: Option<RelayUrl>,
}
```

### API

```rust
impl EmbeddedRelayService {
    pub fn new() -> Self;
    pub async fn start(&mut self, config: EmbeddedRelayConfig) -> Result<RelayUrl>;
    pub async fn stop(&mut self);
    pub fn status(&self) -> &EmbeddedRelayStatus;
    pub fn bound_relay_url(&self) -> Option<&RelayUrl>;
}
```

## Intégration runtime

### Séparation des responsabilités

| Composant | Responsabilité |
|-----------|---------------|
| `RuntimeState` | Décide qu'un relay local est souhaité. Émet `RuntimeEffect`. Reçoit `RuntimeCommand` retour. |
| `RuntimeEffect` | `StartEmbeddedRelay { config }`, `StopEmbeddedRelay` |
| Boucle async | Appelle `service.start()/stop()`. Réinjecte `RuntimeCommand` + émet `ProtocolEvent`. |
| `EmbeddedRelayService` | Encapsule `tom_relay::Server`, status, bound URL. |

### Retour double (contrôle + observabilité)

**RuntimeCommand** (boucle → RuntimeState) :
- `EmbeddedRelayStarted { url: RelayUrl }`
- `EmbeddedRelayFailed { error: String }`
- `EmbeddedRelayStopped`

**ProtocolEvent** (runtime → app) :
- `EmbeddedRelayStarted { url: RelayUrl }`
- `EmbeddedRelayFailed { error: String }`
- `EmbeddedRelayStopped`

### Flux

```
RuntimeState émet StartEmbeddedRelay { config }
    ↓
Boucle async reçoit l'effet
    ↓
Appelle service.start(config)
    ↓
  OK → RuntimeCommand::EmbeddedRelayStarted { url }
       + ProtocolEvent::EmbeddedRelayStarted { url }
  ERR → RuntimeCommand::EmbeddedRelayFailed { error }
        + ProtocolEvent::EmbeddedRelayFailed { error }
    ↓
RuntimeState met à jour son état interne
```

## Séparation rôle logique vs relay réel

- `PeerRole::Relay` = label topologique, gossip-visible
- `EmbeddedRelayStatus::Healthy` = serveur local qui écoute
- Les deux sont **indépendants**
- Jamais l'équation `PeerRole::Relay == relay prêt`

## Tests requis

1. **Start/stop** — démarrage, vérification healthy, arrêt propre
2. **Bind failure** — port déjà pris → `Failed(error)`, pas de panic
3. **Double start** — idempotent ou erreur claire
4. **Non-publication** — aucune annonce gossip/PeerAnnounce automatique
5. **Status observable** — transitions Stopped → Starting → Healthy → Stopped

## Critères d'acceptation

- [ ] Un node peut démarrer un vrai relay embarqué
- [ ] Un node peut l'arrêter proprement
- [ ] Les erreurs de bind sont remontées proprement
- [ ] L'état est observable (status + bound_relay_url)
- [ ] RuntimeState ne contient aucune I/O
- [ ] La boucle async pilote le lifecycle via effects/commands
- [ ] Aucune publication automatique
- [ ] Tests couvrent start, stop, bind failure, non-publication
