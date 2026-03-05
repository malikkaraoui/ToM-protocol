# Option C — Protocole v1.0 → Infrastructure (Plan d’exécution)

> Date: 2026-03-04  
> Scope: Infrastructure production-ready (Relay Fleet + Discovery + Monitoring + Client MVP)  
> Base: protocole feature-complete R1→R14

## 1) Objectif et stratégie d’exécution

Le protocole est fonctionnel. Le risque principal est désormais **infrastructurel**, pas fonctionnel.  
On exécute en **gates successifs**:

1. Relay production (TLS + santé + continuité)
2. Discovery multi-relay côté client
3. Observabilité/alerting MVP
4. Client MVP de validation infra (WASM recommandé)

Règle: on ne passe pas au gate suivant sans validation du gate courant.

---

## 2) Détails clés (issus du brief) à verrouiller

- Multi-relay discovery (pas de relay unique hardcodé)
- TLS obligatoire en production
- Health checks relay + retrait auto des relays down
- Failover automatique côté client
- Monitoring centralisé minimal (Prometheus + Grafana + alertes)
- Éviter cardinalité élevée des métriques
- Priorité à Relay Fleet avant mobile

---

## 3) Pièges critiques (et garde-fous)

## 🔒 Piège A — Invariants wire iroh/tom

À ne jamais casser:
- Préfixes DNS `_iroh`
- SNI `.iroh.invalid`
- Headers `X-Iroh-*`
- ALPN relay/QUIC existant

**Garde-fou**: après chaque changement relay, exécuter test e2e (`tom-stress`) et tests crate `tom-relay`.

## ⚡ Piège B — Stateless != zéro état mémoire

Le relay ne persiste pas de messages mais maintient des connexions en RAM.  
Un restart coupe les sessions.

**Garde-fou**:
- rolling update relay par relay
- arrêt gracieux (drain) avant shutdown
- clients auto-reconnect validés en test

## 🔐 Piège C — TLS prod

Pas de TLS = no-go production.

**Constat code actuel**:
- TLS est déjà supporté via config dans `crates/tom-relay/src/main.rs`
- endpoint santé JSON présent via `GET /healthz` dans `crates/tom-relay/src/server.rs`

**Garde-fou**:
- conserver `--dev` HTTP local
- vérifier certs manuels + Let’s Encrypt staging

## 🌐 Piège D — Discovery SPOF

Si discovery HTTP tombe, bootstrap impossible pour nouveaux clients.

**Garde-fou** (ordre):
1. HTTP discovery (primaire)
2. DNS TXT fallback
3. hardcoded relay list fallback

## 📈 Piège E — explosion cardinalité metrics

Interdit: labels `node_id`, `message_id`, `user_id`.  
Autorisé: `relay`, `region`, `message_type` (faible cardinalité).

---

## 4) Organisation chantier (phases courtes)

## Phase C1 — Relay Fleet MVP (TLS + santé + runbook)

### But
Avoir au moins 1 relay prod robuste, reproductible, avec endpoint santé + métriques.

### Fichiers ciblés (repo)

- `crates/tom-relay/src/main.rs`
  - vérifier/compléter UX CLI/config pour TLS manuel + Let’s Encrypt
  - durcir validations de config startup
- `crates/tom-relay/src/server.rs`
  - conserver `/healthz`, ajouter alias `/health` si nécessaire (compat ops)
  - vérifier que health ne dépend d’aucun état externe fragile
- `deploy/tom-relay.service`
  - paramètres runtime prod, restart policy, arrêt gracieux
- `docs/` (nouveaux docs)
  - guide TLS (manual + LE staging)
  - runbook rolling restart

### Tests/validation

- `cargo test -p tom-relay`
- test smoke HTTP/TLS endpoints (`/healthz`, `/health`, `/metrics`)
- test `tom-stress` contre relay TLS

### Go/No-Go

- [ ] relay TLS démarre de manière déterministe
- [ ] endpoint santé OK
- [ ] métriques exposées
- [ ] e2e message via relay TLS validé

---

## Phase C2 — Discovery service + multi-relay côté client

### But
Un client peut découvrir plusieurs relays et basculer automatiquement.

### Fichiers ciblés (repo)

#### Côté transport/config
- `crates/tom-transport/src/config.rs`
  - étendre `TomNodeConfig` avec:
    - `relay_urls: Vec<RelayUrl>` (multi-relay explicite)
    - `relay_discovery_url: Option<Url>`
    - `relay_discovery_ttl: Duration` (ou `u64` sec)
    - fallback DNS/hardcoded
- `crates/tom-transport/src/node.rs`
  - initialiser `Endpoint` avec map/list de relays (pas un seul)
  - fusionner relays statiques + discovery
- `crates/tom-transport/src/connection.rs`
  - remplacer `default_relay_url: Option<RelayUrl>` par stratégie multi-relay
  - fallback relay rotation en cas d’échec de connexion

#### Côté connect (sélection relay)
- `crates/tom-connect/src/net_report.rs`
- `crates/tom-connect/src/socket/transports/relay/actor.rs`
  - valider le choix “best relay” à partir de pool discovery
  - ne pas casser QAD/probing existant

#### Nouveau composant discovery (dans repo)
- `tools/relay-discovery/` (nouveau dossier)
  - service HTTP JSON minimal:
    - `GET /relays`
    - réponse `{"relays": [...], "ttl_seconds": 300}`
  - health polling des relays
  - exclusion des relays down

#### Schéma JSON canonique

```json
{
  "relays": [
    {
      "url": "https://relay-eu.tom-protocol.org",
      "region": "eu-west",
      "load": 0.3,
      "latency_hint_ms": 50
    }
  ],
  "ttl_seconds": 300
}
```

### Tests/validation

- unit tests parsing/caching TTL
- tests failover client (relay1 down -> relay2)
- campagne `tom-stress` à 2 relays

### Go/No-Go

- [ ] client charge une liste de relays active
- [ ] failover automatique validé
- [ ] fallback discovery activé (HTTP -> DNS -> hardcoded)

---

## Phase C3 — Monitoring & alerting MVP

### But
Voir l’état réel de la flotte et recevoir une alerte utile en cas de panne.

### Fichiers ciblés (repo)

- `deploy/monitoring/prometheus.yml` (nouveau)
  - scrape des relays (`/metrics`)
- `deploy/monitoring/alerts.yml` (nouveau)
  - règles:
    - relay down > 2min
    - latence p95 > 500ms pendant 5min
    - taux d’erreur > 5%
- `deploy/monitoring/grafana-dashboard-option-c.json` (nouveau)
  - panels: connexions, débit relay, p95, up/down
- `docs/monitoring/option-c-monitoring.md` (nouveau)
  - déploiement + runbook alerte

### Requêtes/labels

- autoriser labels: `relay`, `region`
- interdire labels haute cardinalité (node/message IDs)

### Go/No-Go

- [ ] dashboards lisibles
- [ ] alerte relay down testée
- [ ] alerte latence testée

---

## Phase C4 — Client MVP de validation infra

### But
Prouver qu’un client “réel” consomme l’infra multi-relay.

### Option recommandée
Option B (WASM/Web) pour MVP rapide.

### Fichiers ciblés (repo)

- `crates/tom-wasm/` (nouveau, si retenu)
- `apps/infra-web-client/` (nouveau)
- documentation intégration (`docs/clients/wasm-mvp.md`)

### Go/No-Go

- [ ] envoi/réception sur infra discovery
- [ ] test cross-region (EU -> US)

---

## 5) Ordre de livraison (proposé)

1. **C1** Relay prod prêt
2. **C2** Discovery + failover
3. **C3** Monitoring + alertes
4. **C4** Client MVP

Pas de saut d’étape.

---

## 6) Découpage tickets “fichier par fichier” (micro-sessions)

### Ticket C1.1 — Durcir startup relay
- `crates/tom-relay/src/main.rs`
- Output: validation config explicite + messages d’erreur ops

### Ticket C1.2 — Endpoint santé standardisé
- `crates/tom-relay/src/server.rs`
- Output: `/healthz` + alias `/health` (si choisi)

### Ticket C1.3 — Service systemd robuste
- `deploy/tom-relay.service`
- Output: restart policy + timeout/graceful stop

### Ticket C2.1 — Config multi-relay
- `crates/tom-transport/src/config.rs`
- Output: `relay_urls`, `relay_discovery_url`, TTL cache

### Ticket C2.2 — Node bind avec pool relays
- `crates/tom-transport/src/node.rs`
- Output: builder supporte multi-relay + fallback

### Ticket C2.3 — Connection failover
- `crates/tom-transport/src/connection.rs`
- Output: retry/rotation relay sans casser API publique

### Ticket C2.4 — Discovery service minimal
- `tools/relay-discovery/*`
- Output: endpoint `/relays` + health polling

### Ticket C2.5 — E2E failover
- `crates/tom-stress/src/*` (scénario dédié)
- Output: test relay1 down -> relay2

### Ticket C3.1 — Prometheus + alerts
- `deploy/monitoring/prometheus.yml`
- `deploy/monitoring/alerts.yml`

### Ticket C3.2 — Dashboard Grafana
- `deploy/monitoring/grafana-dashboard-option-c.json`

### Ticket C4.1 — Client validation infra
- `crates/tom-wasm/*` + `apps/infra-web-client/*` (si option web)

---

## 7) Critères “Infra Ready” (MVP)

- [ ] ≥ 3 relays TLS (multi-régions)
- [ ] discovery retourne uniquement relays sains
- [ ] failover client automatique validé
- [ ] dashboards + alertes opérationnels
- [ ] 1 client fonctionnel sur cette infra
- [ ] test cross-region réussi

---

## 8) Commandes de validation (référence équipe)

- `cargo test -p tom-relay`
- `cargo test -p tom-transport`
- `cargo test -p tom-connect`
- `cargo test -p tom-stress`
- `cargo test --workspace`

Campagnes:
- `tom-stress` local multi-relay
- campagne cross-region (EU/US)

---

## 9) Notes de compatibilité projet

- Conserver les invariants wire listés dans `CLAUDE.md`.
- Ne pas introduire de persistance message dans relay (stateless forwarding conservé).
- Éviter tout changement non nécessaire des APIs publiques.
- Prioriser petits commits testables par ticket.
