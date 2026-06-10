# ToM Dev Metrics & Dashboard — Plan d'exécution V2

> Date : 2026-06-09 (révision)
> Statut : **Exécutable**
> Scope : dev mode uniquement — aucun impact production

---

## 1. Verdict sur le design précédent

### Ce qu'on garde

- L'idée centrale : polling HTTP local vers des nœuds exposant leur état
- Les métriques identifiées (identité, réseau, messages, groupes)
- La contrainte iOS/tvOS sandbox (nœuds sandboxés ne peuvent pas exposer de port)

### Ce qu'on jette et pourquoi

| Élément jeté | Raison |
|---|---|
| Nouveau crate `tom-dashboard` | `spawn_status_server()` existe déjà dans `tom-tui/src/main.rs` — ~60 lignes de TCP/HTTP brut qui sérialisent `MetricsSnapshot`. Créer un crate Axum pour ça est une sur-ingénierie pure. |
| Nouveau frontend `tools/dev-dashboard` | `apps/infra-web-client/` existe déjà : Vite TS, `probeJson()` multi-endpoints, render HTML. Il manque juste la section nœuds. |
| `/nodes` et `POST /heartbeat` dans `tom-relay` | Le relay est stateless par design (ADR-001). Lui donner de la mémoire applicative viole l'architecture. Les métriques du relay (`tom-relay/src/server/metrics.rs`) sont transport-level (bytes, paquets) — sans lien avec la topologie applicative. |
| SSE / Server-Sent Events | Du polling à 5s suffit largement pour du monitoring de dev. SSE ajoute une connexion persistante, un handler async de plus, une gestion de reconnect côté client. Ratio complexité/valeur = 0. |
| iOS/tvOS heartbeat FFI en V1 | La contrainte sandbox est réelle. Mais envoyer des métriques applicatives via le relay (infrastructure réseau) crée un couplage inter-couches. En V1 ce n'est pas nécessaire — les nœuds desktop suffisent pour diagnostiquer le réseau. |
| Contrat JSON non versionné | L'endpoint actuel n'a pas `schema_version`. À corriger, mais pas en créant un nouveau serveur — juste en modifiant le format existant. |

---

## 2. Architecture V1 recommandée

```
apps/infra-web-client (localhost:5173 — EXISTANT)
   │  polling GET /status toutes les 5s
   ├──→ localhost:8080/status     (MacBook — tom-tui --status-port 8080)
   ├──→ 192.168.0.21:9090/status  (NAS — tom-tui --status-port 9090)
   └──→ [autres nœuds configurables dans l'UI]

tom-relay (82.67.95.8:3340) : INCHANGÉ
   └── métriques transport déjà dans server/metrics.rs (iroh_metrics)
```

**Sources de données :**
- `spawn_status_server()` dans `crates/tom-tui/src/main.rs` — déjà fonctionnel, expose JSON sur TCP brut
- `RuntimeHandle::metrics()` + `handle.connected_peers()` + `handle.groups()` — déjà appelés par `spawn_status_server`
- `infra-web-client/src/main.ts` — déjà capable de `probeJson()` vers plusieurs endpoints

**Nouveautés réelles :**
1. Ajouter `schema_version: 1` + `platform` + `relay_url_active` au JSON du status server (2 lignes Rust)
2. Ajouter une section "Nœuds" dans `infra-web-client` qui rend les cartes par nœud (50-70 lignes TS + HTML)
3. Rendre les endpoints configurables dans l'UI (déjà fait pour relay/discovery — même pattern)

**Cas iOS/tvOS :**
Hors scope V1. En V2, si nécessaire : un processus collecteur séparé (pas le relay) reçoit les heartbeats push. Voir §4.

---

## 3. Contrat JSON V1

L'endpoint `GET /status` (ou n'importe quel chemin — le serveur actuel répond à tout GET) retourne :

```json
{
  "schema_version": 1,
  "node": "nas-bot",
  "node_id": "ab12cd34ef56...",
  "platform": "linux",
  "relay_url_active": "http://192.168.0.21:3340",
  "phase": "Converged",
  "taille_reseau": 3,
  "role": "Peer",
  "relayeurs": 1,
  "pairs_connectes": ["ab12cd34", "ef56gh78"],
  "groupes": [
    { "nom": "famille", "membres": 3 }
  ],
  "messages_envoyes": 42,
  "messages_recus": 38,
  "messages_echoues": 0,
  "uptime_secondes": 3600
}
```

**Champs obligatoires :** `schema_version`, `node`, `node_id`, `phase`, `taille_reseau`, `role`, `uptime_secondes`

**Champs optionnels (peuvent être absents ou null) :** `platform`, `relay_url_active`, `relayeurs`, `pairs_connectes`, `groupes`, `messages_envoyes`, `messages_recus`, `messages_echoues`

**Règle de compatibilité :** si `schema_version` absent → ancien format, le client affiche les champs disponibles sans erreur. Si `schema_version >= 2` → le client signale "version non supportée" dans la carte.

**Champs non inclus en V1 (coût trop élevé ou données indisponibles sans refactoring) :**
- `rtt_ms_per_peer` — nécessite exposition de l'état interne de MagicSock, pas surfacé dans `MetricsSnapshot`
- `cpu_pct`, `battery_pct` — APIs système non intégrées dans le runtime
- `hole_punch_success/fail` — compteurs dans tom-connect non remontés dans `ProtocolMetrics`
- `msgs_in_flight`, `msgs_acked` — MessageTracker non exposé dans `MetricsSnapshot`

---

## 4. Périmètre V1 / V2 / non retenu

| Fonctionnalité | Scope | Justification |
|---|---|---|
| `schema_version` dans le JSON existant | **V1** | 1 ligne Rust |
| `platform` + `relay_url_active` dans le JSON | **V1** | 2 lignes Rust, déjà disponibles via `cfg!` et `std::env` |
| Section "Nœuds" dans infra-web-client | **V1** | 50-70 lignes TS, infra `probeJson` déjà là |
| Endpoints configurables dans l'UI (NAS, Mac, autres) | **V1** | Pattern déjà fait pour relay/discovery |
| Indicateurs santé colorés par nœud (vert/jaune/rouge) | **V1** | Logique triviale sur `taille_reseau` et `phase` |
| Polling 5s auto-refresh | **V1** | Déjà fait pour relay/discovery dans infra-web-client |
| iOS/tvOS heartbeat push | **V2** | Nécessite un collecteur séparé (pas relay), FFI Swift, Timer 30s |
| `rtt_ms_per_peer` | **V2** | Nécessite exposition MagicSock → ProtocolMetrics |
| `msgs_in_flight`, `msgs_acked` | **V2** | Nécessite exposition MessageTracker |
| Métriques système (CPU, RAM, batterie) | **V2** | APIs platform-specific, coût iOS/macOS non négligeable |
| Sparklines historique 5 min | **V2** | Buffer RAM côté client, Chart.js — pas bloquant mais pas V1 |
| Collecteur central (agréger N nœuds en un seul JSON) | **Non retenu** | Le polling direct depuis le browser suffit pour dev ; un collecteur crée un SPOF et un processus de plus |
| SSE | **Non retenu** | Polling 5s = acceptable pour du monitoring dev ; SSE ajoute complexité sans valeur visible |
| `/nodes` et `/heartbeat` dans tom-relay | **Non retenu** | Viole le principe stateless du relay (ADR-001) |
| Nouveau crate `tom-dashboard` | **Non retenu** | `spawn_status_server` + `infra-web-client` couvrent le besoin |
| Auth token sur le status server | **V2** | En dev LAN, acceptable sans auth. Si exposition WAN : token query param ou header fixe suffisent |

---

## 5. Plan d'implémentation

### Étape 1 — `crates/tom-tui/src/main.rs` : enrichir le JSON (15 min)

**Fichier :** `crates/tom-tui/src/main.rs`, fonction `spawn_status_server()` (ligne ~105)

Modifications :
- Ajouter `"schema_version":1` au début du body
- Ajouter `"platform":"..."` via `cfg!(target_os = ...)`
- Ajouter `"relay_url_active":"..."` via `std::env::var("TOM_RELAY_URL")`

Impact crate : `tom-tui` uniquement. Pas de changement dans `tom-protocol`.

```rust
// Dans le format! de spawn_status_server, ajouter en tête :
"schema_version":1,
"platform":"{platform}",
"relay_url_active":"{relay}",

// Dans les bindings :
platform = if cfg!(target_os = "macos") { "macos" }
           else if cfg!(target_os = "linux") { "linux" }
           else { "unknown" },
relay = std::env::var("TOM_RELAY_URL").unwrap_or_default(),
```

Pas de nouveau type, pas de dépendance ajoutée.

### Étape 2 — `apps/infra-web-client/src/main.ts` : section nœuds (1-2h)

**Fichier :** `apps/infra-web-client/src/main.ts`

Ajouter :
1. Un tableau `NODE_ENDPOINTS` configurable (URLs saisies dans l'UI, persistées en `localStorage`)
2. Une fonction `renderNodeCard(url, result)` qui affiche une carte par nœud
3. Dans `refresh()` : `Promise.all` sur tous les endpoints nœuds, appel `renderNodeCard`

La fonction `probeJson()` existante est réutilisée telle quelle.

Logique santé :
```typescript
function nodeHealth(data: NodeStatus): 'ok' | 'warn' | 'ko' {
  if (!data) return 'ko';
  if (data.taille_reseau >= 2 && data.phase === 'Converged') return 'ok';
  if (data.taille_reseau >= 1) return 'warn';
  return 'ko';
}
```

### Étape 3 — `apps/infra-web-client/index.html` : section nœuds (20 min)

Ajouter une section `<div id="nodesSection">` avec :
- Un input pour ajouter des endpoints
- Un conteneur `<div id="nodeCards">` rempli par `renderNodeCard()`

Pas de nouvelle dépendance npm. Pas de build step supplémentaire.

### Ordre d'exécution conseillé

1. Étape 1 (Rust) — valider avec `cargo build -p tom-tui` + test manuel `curl localhost:8080`
2. Étape 2 + 3 (TS) — `pnpm dev` dans `apps/infra-web-client/`
3. Smoke test multi-nœud (Mac + NAS si accessible)

---

## 6. Validation

### Unit tests
- Pas de nouveau test unitaire nécessaire : `spawn_status_server` n'a pas de logique métier testable séparément
- `MetricsSnapshot` a déjà des tests dans `crates/tom-protocol/src/runtime/metrics.rs` (`metrics_snapshot_serializes`) — vérifier que `serde_json::to_string` continue à passer

### Cross-crate build
```bash
cargo build -p tom-tui
cargo clippy -p tom-tui -- -D warnings
```
Pas de changement dans `tom-protocol`, `tom-connect`, `tom-relay` → pas de cross-crate.

### Smoke local
```bash
# Terminal 1 : nœud Mac avec status port
cargo run -p tom-tui -- --username malik --status-port 8080

# Vérifier le JSON
curl -s http://localhost:8080 | jq .

# Terminal 2 : frontend
cd apps/infra-web-client && pnpm dev
# → ouvrir http://localhost:5173, ajouter http://localhost:8080 comme endpoint nœud
```

### Validation multi-device
- NAS : `tom-tui --status-port 9090` → vérifier depuis Mac `curl http://192.168.0.21:9090 | jq .`
- Ajouter `http://192.168.0.21:9090` dans l'UI infra-web-client
- Critère de succès : les deux cartes (Mac + NAS) s'affichent avec `phase: Converged` et `taille_reseau >= 2`

---

## 7. Risques

| Risque | Probabilité | Impact | Mitigation |
|---|---|---|---|
| **CORS** : le browser bloque les requêtes vers `192.168.0.21:9090` | Haute | Bloquant | `spawn_status_server` doit ajouter `Access-Control-Allow-Origin: *` dans la réponse HTTP. À faire dans l'étape 1. |
| **Contrat JSON instable** : si `spawn_status_server` est modifié plus tard, le frontend casse silencieusement | Moyenne | Moyen | `schema_version` + parsing défensif côté TS (`data?.taille_reseau ?? 0`) |
| **iOS/tvOS absent du dashboard** | Certaine | Faible (dev) | Documenté explicitement. V2 si besoin. |
| **NAS inaccessible** (observé aujourd'hui) | Régulière | Faible | La carte nœud s'affiche en rouge avec le message d'erreur réseau — comportement attendu |
| **`--status-port` exposé sur `0.0.0.0`** | Certaine | Faible si LAN uniquement | Le serveur actuel bind sur `0.0.0.0`. En dev LAN c'est acceptable. Ne pas activer sur une machine exposée sur Internet sans firewall. |

---

## Annexe — Ce que ce plan ne fait pas

- Pas de persistance disque
- Pas de collecte en prod (`--status-port` = flag dev, à ne pas passer en production)
- Pas de centralisation des logs (métriques numériques uniquement)
- Pas de control plane (dashboard read-only)
- Pas de support multi-relay
- Pas de websocket, pas de SSE
