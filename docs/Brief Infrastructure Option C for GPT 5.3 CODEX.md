Brief Infrastructure (Option C) pour GPT 5.3 Codex
Vue d'ensemble
Objectif : Passer du protocole feature-complete (R1-R14) à une infrastructure production-ready :

Relay Fleet : N relays géographiquement distribués, load-balanced
Monitoring : Observabilité, metrics, alerting
Clients : Mobile (iOS/Android) + Web
Priorité : Relay Fleet d'abord (sans relay robuste, rien ne fonctionne en prod).

Partie 1 : Relay Fleet Production
Scope exact
Ce qui est déjà fait :

✅ tom-relay (crate Rust, fork iroh-relay)
✅ Mode --dev (HTTP, port 3340, single instance)
✅ Déployé sur Freebox NAS (test interne)
Ce qui manque :

Multi-relay discovery : Clients doivent découvrir N relays (pas juste 1 URL hardcodée)
Geographic distribution : Relays dans plusieurs régions (latence réduite)
Load balancing : Client choisit le relay le plus proche/rapide
Health checks : Relays morts retirés automatiquement de la pool
TLS/HTTPS : Production = obligatoire (actuellement HTTP only en --dev)
Metrics export : Prometheus endpoints déjà présents, mais pas agrégés
Architecture recommandée

┌─────────────────────────────────────────────────────┐
│ Relay Discovery Service (DNS ou HTTP API)          │
│ ├─ Retourne liste de relays actifs                 │
│ └─ Format: [{url, region, load, latency_hint}]     │
└─────────────────────────────────────────────────────┘
                         ↓
        ┌────────────────┼────────────────┐
        ↓                ↓                ↓
   [Relay EU]       [Relay US]       [Relay ASIA]
   ├─ TLS cert      ├─ TLS cert      ├─ TLS cert
   ├─ Metrics       ├─ Metrics       ├─ Metrics
   └─ Health /ping  └─ Health /ping  └─ Health /ping
Client-side :

TomNodeConfig::relay_discovery_url() → fetch relay list
TomNodeConfig::builder() inject multiple relay URLs (pas juste 1)
MagicSock connecte au relay le plus rapide (QAD probe déjà fait ça)
Server-side :

Chaque relay expose /health (200 OK si healthy)
Discovery service poll /health toutes les 30s
Discovery service retourne seulement relays healthy
Pièges à éviter
🚨 Piège 1 : Wire protocol invariants
CRITIQUE : Les relays utilisent le protocole iroh wire (headers X-Iroh-*, TLS SNI .iroh.invalid).

NE JAMAIS :

Changer les headers HTTP (X-Iroh-NodeId, X-Iroh-Version)
Changer le TLS SNI (.iroh.invalid)
Changer le format des frames relay
Pourquoi ? : tom-connect (MagicSock) attend ces invariants. Les casser = clients ne peuvent plus se connecter.

Validation : Après chaque modif relay, tester avec tom-stress ping (end-to-end).

🚨 Piège 2 : Relay stateless != Relay sans état mémoire
Les relays sont stateless (pas de persistence SQLite), mais ont état RAM :

Connexions actives (WebSocket ou QUIC)
Routing table (NodeId → Connection)
Piège : Restart relay = tous les clients perdent connexion, doivent re-connect.

Mitigation :

Rolling updates (1 relay à la fois)
Clients auto-reconnect (déjà implémenté dans MagicSock)
Grace period avant shutdown (SIGTERM → drain connections 30s → SIGKILL)
🚨 Piège 3 : TLS certificates
Mode dev : HTTP, pas de TLS → OK pour tests LAN.

Production : HTTPS obligatoire, sinon :

Les navigateurs bloquent (Mixed Content)
Les ISPs peuvent inspecter/bloquer le trafic
Solutions :

Let's Encrypt : Certs gratuits, auto-renewal (recommandé)
Wildcard cert : *.relay.tom-protocol.org → tous les relays partagent le même cert
Self-signed : NON (clients doivent trust le CA manuellement → UX horrible)
Piège : tom-relay utilise rustls (pas OpenSSL). Vérifier compatibilité Let's Encrypt → rustls.

Code existant : tom-relay a déjà du code TLS (commenté en mode --dev). GPT 5.3 doit :

Uncomment le code TLS
Ajouter CLI flags --cert et --key
Tester avec un cert Let's Encrypt staging (pas prod, pour éviter rate limits)
🚨 Piège 4 : Discovery service = single point of failure
Risque : Si discovery service est down, les clients ne trouvent aucun relay → réseau paralysé.

Mitigation :

Fallback list hardcodée : Si discovery échoue, client utilise une liste de relays publics hardcodés
DNS-based discovery : Plus résilient que HTTP (anycast, TTL cache)
Gossip-based discovery : Clients partagent leurs relays via gossip (déjà implémenté !)
Recommandation : Hybrid approach

Primary : HTTP discovery service
Fallback 1 : DNS TXT records (_relay._tcp.tom-protocol.org)
Fallback 2 : Hardcoded list (3-5 relays publics)
Conseils d'implémentation
Conseil 1 : Commencer MVP (1 relay prod, TLS)
Phase 1 (2-3h) :

Déployer tom-relay sur un VPS (Hetzner, DigitalOcean, AWS)
Enable TLS avec Let's Encrypt
DNS : relay1.tom-protocol.org → <VPS_IP>
Test : tom-stress ping --relay-url https://relay1.tom-protocol.org
Validation : Si ping fonctionne, relay prod = OK.

Puis : Ajouter relays 2, 3, N (Europe, US, Asia).

Conseil 2 : Discovery service = simple HTTP JSON
Spec minimal :


GET https://discovery.tom-protocol.org/relays
Response :


{
  "relays": [
    {
      "url": "https://relay-eu.tom-protocol.org",
      "region": "eu-west",
      "load": 0.3,
      "latency_hint_ms": 50
    },
    {
      "url": "https://relay-us.tom-protocol.org",
      "region": "us-east",
      "load": 0.5,
      "latency_hint_ms": 120
    }
  ],
  "ttl_seconds": 300
}
Client-side :

Fetch toutes les 5min (TTL)
Cache localement
Probe latency (ping chaque relay)
Connecte au plus rapide
Server-side (discovery service) :

Liste statique (JSON file) → simple, pas de DB
Health check poll toutes les 30s
Retire relays down de la liste
Tech stack : Python Flask (50 lignes), Rust Axum (100 lignes), ou même static JSON sur CDN.

Conseil 3 : Metrics aggregation (Prometheus + Grafana)
tom-relay expose déjà /metrics (Prometheus format). Il faut :

Prometheus scrape config :

scrape_configs:
  - job_name: 'tom-relay'
    static_configs:
      - targets:
        - 'relay-eu.tom-protocol.org:9090'
        - 'relay-us.tom-protocol.org:9090'
Grafana dashboards :
Connexions actives (gauge)
Messages relayés/s (counter)
Latency p50/p95/p99 (histogram)
Relays down (alert)
Piège : tom-relay metrics port ≠ relay port (ex: relay 3340, metrics 9090).

Validation : curl https://relay-eu.tom-protocol.org:9090/metrics doit retourner du texte Prometheus.

Niveau de détails pour GPT 5.3
Donne-lui :

Spec complète du endpoint discovery (format JSON exact, comportement cache)
Wire invariants (headers, SNI, frames) → copier-coller la section "Wire Invariants" de CLAUDE.md
TLS requirements : Let's Encrypt, rustls, CLI flags --cert/--key
Health check endpoint : GET /health → 200 OK (30 secondes timeout)
Metrics aggregation : Prometheus scrape config exemple
Ne lui donne PAS :

Décisions d'hébergement (VPS provider, régions géographiques) → toi tu décides
Design UX discovery service (UI web ?) → hors scope protocole
Scaling complexe (Kubernetes, auto-scaling) → overkill pour MVP
Attends de lui :

Code TLS pour tom-relay (uncomment + test)
Discovery service (HTTP JSON, 50-100 lignes)
Client-side relay selection (TomNodeConfig multi-relay)
Health check endpoint dans tom-relay
Tests end-to-end (2 relays, client ping via relay 1 puis failover relay 2)
Partie 2 : Monitoring & Observability
Scope exact
Ce qui existe :

✅ tom-metrics (Counter, Gauge)
✅ Prometheus endpoints dans tom-relay
✅ ProtocolMetrics dans tom-protocol (messages sent/received/failed, uptime)
Ce qui manque :

Centralized logging : Agréger logs de tous les relays (Loki, ElasticSearch ?)
Distributed tracing : Trace un message de sender → relay → recipient (OpenTelemetry ?)
Alerting : Slack/PagerDuty si relay down, latency >500ms, error rate >5%
Dashboards : Grafana prêt à l'emploi (pas juste metrics bruts)
Architecture recommandée

Relays → Prometheus (scrape /metrics)
       → Loki (logs)
       → Jaeger (traces optionnel)
              ↓
         Grafana (dashboards + alerting)
              ↓
         Slack/PagerDuty (notifications)
Minimal viable :

Prometheus + Grafana (déjà standard)
Skip tracing (complexe, pas critique pour MVP)
Logs agrégés = nice-to-have (peut attendre)
Pièges à éviter
🚨 Piège 5 : Metrics ≠ Logs ≠ Traces
Metrics : Compteurs, gauges, histograms (ex: "messages relayés/s")
Logs : Événements textuels (ex: "ERROR: relay timeout NodeId xyz")
Traces : Propagation d'un request ID à travers N services

Pour MVP : Metrics suffisent. Logs et traces = optimisation post-MVP.

Conseil à GPT 5.3 : Focus sur Prometheus + Grafana. Ignorer Loki/Jaeger pour l'instant.

🚨 Piège 6 : Metrics explosion (cardinality)
Risque : Si tu track messages_sent{node_id="..."}, et qu'il y a 1M de nodes → 1M de time series → Prometheus OOM.

Règle : Pas de labels à haute cardinalité (node_id, message_id, user_id).

OK :

relay_connections_active{region="eu"}
messages_relayed_total{relay="relay-eu"}
PAS OK :

messages_sent{from="node123", to="node456"} (cardinalité = N²)
Conseil à GPT 5.3 : Limiter labels à region, relay_id, message_type (faible cardinalité).

Conseils d'implémentation
Conseil 4 : Grafana dashboard template
Donne à GPT 5.3 un template JSON de dashboard Grafana avec :

Panel 1 : Connexions actives (gauge)
Panel 2 : Messages relayés/s (graph)
Panel 3 : Relays up/down (status map)
Panel 4 : Latency p95 (graph)
Il peut : Générer le JSON Grafana via code (Python script qui call Grafana API).

Validation : Import JSON dans Grafana → dashboard s'affiche correctement.

Niveau de détails pour GPT 5.3
Donne-lui :

Prometheus scrape config complet (targets, intervals, labels)
Grafana dashboard spec (panels, queries PromQL)
Alert rules (relay down >2min, latency >500ms sustained 5min)
Cardinality limits (pas de node_id dans labels)
Ne lui donne PAS :

Infrastructure Prometheus (self-hosted vs Grafana Cloud) → toi tu décides
Design dashboards UX (couleurs, layout) → template suffit
Attends de lui :

Prometheus scrape config (YAML)
Grafana dashboard (JSON export)
Alert rules (YAML ou Grafana format)
Documentation "How to deploy monitoring stack"
Partie 3 : Clients (Mobile + Web)
Scope exact
Ce qui existe :

✅ tom-tui (TUI chat client, Rust)
✅ apps/demo (browser demo, vanilla HTML/JS + Vite)
✅ packages/sdk (TypeScript SDK, Phase 1)
Ce qui manque :

Mobile apps : iOS (Swift) + Android (Kotlin)
Web client production : React/Vue app (pas juste demo vanilla)
FFI bindings : Rust protocol → C FFI → Swift/Kotlin
Architecture recommandée

┌──────────────────────────────────────────┐
│ tom-protocol (Rust)                      │
│ ├─ ProtocolRuntime                       │
│ ├─ TomNodeConfig                         │
│ └─ RuntimeHandle (send/receive)          │
└──────────────────────────────────────────┘
              ↓
┌──────────────────────────────────────────┐
│ tom-ffi (C FFI bindings)                 │
│ ├─ tom_node_new()                        │
│ ├─ tom_send_message()                    │
│ └─ tom_on_message(callback)              │
└──────────────────────────────────────────┘
       ↓                    ↓
  [Swift iOS]         [Kotlin Android]
  └─ TomClient.swift  └─ TomClient.kt
Alternative : UniFFI (Mozilla) génère automatiquement Swift/Kotlin bindings depuis Rust.

Recommandation : UniFFI (moins de boilerplate que FFI manuel).

Pièges à éviter
🚨 Piège 7 : Async Rust → Sync FFI
Rust tokio (async) ne passe PAS directement en FFI (C attend du sync).

Mauvais :


#[no_mangle]
pub extern "C" fn tom_send_message(...) -> Result<(), Error> {
    runtime.send_message(...).await // ❌ await dans FFI = compile error
}
Bon :


#[no_mangle]
pub extern "C" fn tom_send_message(..., callback: extern "C" fn()) {
    let runtime = get_runtime();
    tokio::spawn(async move {
        runtime.send_message(...).await;
        callback(); // notify caller
    });
}
Ou mieux : UniFFI gère ça automatiquement (async Rust → async Swift/Kotlin).

🚨 Piège 8 : Callbacks FFI = lifetime hell
Risque : Swift/Kotlin callback → Rust, mais le callback est freed avant d'être appelé → crash.

Solution UniFFI : Utilise Arc<dyn Trait> + trait objects pour callbacks safe.

Conseil à GPT 5.3 : Utilise UniFFI, pas FFI manuel (sauf si raison spécifique).

🚨 Piège 9 : Mobile = background execution limits
iOS/Android tuent les apps en background après quelques minutes.

Problème : Si ProtocolRuntime tourne dans l'app, il meurt en background → pas de messages reçus.

Solutions :

Push notifications : Server envoie push, app wake up, fetch messages
Background fetch : iOS/Android permettent ~15min de fetch périodique
Foreground service : Android permet service persistant (mais battery drain)
Pour MVP : Pas de background execution (app doit être ouverte pour recevoir messages).

Post-MVP : Push notifications (nécessite serveur central → complexe).

Conseils d'implémentation
Conseil 5 : UniFFI workflow
Phase 1 : FFI layer (Rust) (~4-6h)

Créer crate tom-ffi (dépend de tom-protocol)
Définir interface UniFFI :

#[uniffi::export]
fn tom_node_new(relay_url: String) -> Arc<TomNode>;

#[uniffi::export]
fn tom_send_message(node: Arc<TomNode>, to: String, msg: String);

#[uniffi::export]
trait TomNodeDelegate {
    fn on_message(&self, from: String, msg: String);
}
Build : cargo build --release → génère .a (iOS) et .so (Android)
Phase 2 : Swift client (iOS) (~8-10h)

UniFFI génère TomClient.swift
SwiftUI app :

class TomViewModel: ObservableObject, TomNodeDelegate {
    @Published var messages: [Message] = []
    
    func onMessage(from: String, msg: String) {
        messages.append(Message(from: from, text: msg))
    }
}
UI : Liste messages + input field
Phase 3 : Kotlin client (Android) (~8-10h)
Similaire à iOS, avec Jetpack Compose.

Conseil 6 : Web client = WASM
Alternative mobile : Web app (PWA) compilé en WASM.

Avantage :

1 seul codebase (Rust → WASM → web)
Pas de FFI bindings
Deploy facile (static site)
Inconvénient :

Pas d'accès natif (contacts, camera, push notifications limitées)
Performance légèrement inférieure
Pour MVP : Web client WASM peut remplacer iOS/Android (rapid prototyping).

Workflow :

Compile tom-protocol en WASM (wasm-pack)
JS bindings auto-générés
React app appelle tom_send_message() via JS
Conseil à GPT 5.3 : Si pressed for time, WASM web client > native apps (plus rapide à faire).

Niveau de détails pour GPT 5.3
Donne-lui :

UniFFI tutorial (Mozilla docs)
Interface Rust souhaitée (fonctions exposées : new, send, receive, shutdown)
Callback contract (on_message, on_error, on_connection_status)
Build instructions (cargo build → .a/.so → Xcode/Android Studio)
Si WASM :

wasm-pack tutorial
JS API surface (TomClient class, send(), on('message', cb))
React example (hooks useEffect pour subscription)
Ne lui donne PAS :

Design UI (couleurs, layout) → hors scope protocole
App Store submission (certificates, provisioning) → toi tu gères
Backend notifications (push server) → phase 2
Attends de lui :

Crate tom-ffi avec UniFFI bindings
OU crate tom-wasm avec WASM bindings
Example Swift project (iOS) OU React app (web)
Documentation "How to integrate tom-ffi in Xcode/Android Studio"
Résumé : Brief complet pour GPT 5.3
Priorité 1 : Relay Fleet (MVP)
 TLS support dans tom-relay (Let's Encrypt, CLI flags)
 Health check endpoint /health
 Discovery service (HTTP JSON, liste relays actifs)
 Multi-relay support dans TomNodeConfig
 Tests end-to-end (2 relays, failover)
Durée estimée : 6-8h

Priorité 2 : Monitoring (MVP)
 Prometheus scrape config (tous les relays)
 Grafana dashboard template (connexions, messages, latency)
 Alert rules (relay down, latency >500ms)
Durée estimée : 3-4h

Priorité 3 : Client (Choix)
Option A : Native mobile (UniFFI)

 Crate tom-ffi avec UniFFI bindings
 Swift iOS app (SwiftUI)
 Kotlin Android app (Jetpack Compose)
Durée estimée : 16-20h

Option B : Web client (WASM)

 Crate tom-wasm avec wasm-pack
 React app (chat UI)
Durée estimée : 8-10h

Recommandation : Option B (WASM) pour MVP (plus rapide, 1 codebase).

Checklist validation finale
Avant de dire "infra prod ready" :

 3+ relays déployés (EU, US, ASIA) avec TLS
 Discovery service retourne relays actifs
 Client auto-failover si relay down
 Grafana dashboard affiche metrics temps réel
 Alert Slack si relay down >2min
 1 client fonctionnel (WASM web OU iOS OU Android)
 Test end-to-end : envoyer message cross-region (EU → US) via relay
Si tous ✅ → ToM Protocol v1.0 Production Ready 🎉