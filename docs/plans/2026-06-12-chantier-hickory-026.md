# Chantier hickory 0.26 — Suivi d'exécution

> Démarré : 2026-06-12 08:20 · Référence : triage advisories (journal S0 §advisories)
> Objectif : corriger les 2 dernières advisories majeures du triage —
> RUSTSEC-2026-0119 (DoS O(n²) name compression) et RUSTSEC-2026-0118
> (boucle infinie validation NSEC3) — par bump majeur hickory 0.24/0.25 → 0.26.1.

## Tableau de bord

| Tâche | Description | Statut |
|---|---|---|
| H.0 | Cartographie de l'impact (3 crates consommateurs) | ✅ |
| H.1 | Bump manifests + lock racine (hickory 0.26.1) | ✅ |
| H.2 | Adaptation API tom-relay (dns.rs, endpoint_info.rs, server.rs) | ✅ |
| H.3 | Adaptation API tom-connect (test_utils) + tom-transport (node.rs) | ✅ |
| H.4 | Bump swarm-discovery 0.5 → 0.6.1 (purge hickory-proto 0.25.2 résiduel) | ✅ |
| H.5 | deny.toml : retrait des 2 ignores + lock FFI resynchronisé | ✅ |
| H.V | Gates finales + PR + review Copilot | ⏳ |

## Journal de chantier

### Cartographie (Phase 0)

Consommateurs directs de hickory-resolver :
- `tom-relay` 0.25 (features tokio, https-ring) — dns.rs (resolver complet), endpoint_info.rs (TxtLookup)
- `tom-connect` 0.25 — test_utils (serveur DNS de test), + swarm-discovery 0.5 (mDNS) qui tirait hickory-proto **0.25.2**
- `tom-transport` 0.24 — node.rs (1 call site)

### Ruptures d'API 0.25 → 0.26 rencontrées (15 erreurs de compile)

| Ancienne API | Nouvelle API 0.26 |
|---|---|
| `name_server::TokioConnectionProvider` | `TokioRuntimeProvider` (module name_server privatisé) |
| `hickory_resolver::ResolveError` | `hickory_resolver::net::NetError` |
| `proto::xfer::Protocol` | `net::xfer::Protocol` |
| `Lookup` itérable directement | `.answers()` + pattern matching sur RData |
| `ResolverConfig::google()` / `::new()` | constructeurs remplacés (builder) |
| `NameServerConfig::new(socket_addr, proto)` | `new(ip, trust, Vec<ConnectionConfig>)` — **le port se règle sur ConnectionConfig** |
| `Message.queries()` méthode | champ `.queries` direct |
| `TokioAsyncResolver::tokio_from_system_conf()` | `TokioResolver` + builder |

### Incident 1 — disque saturé à 99 % pendant les tests

`target/` du workspace avait gonflé à **62,8 GiB** (builds accumulés des chantiers
S0→S3 + PR sécurité). Le harnais ne pouvait même plus écrire ses sorties de
commandes. Résolution : `cargo clean` (validé avec Malik), 38 Gi récupérés,
recompilation complète ensuite. Backlog : purge périodique de target/.

### Incident 2 — 3 tests DNS en échec après la migration initiale

`address_lookup::test_dns_pkarr::{dns_resolve, pkarr_publish_dns_resolve, pkarr_publish_dns_discover}`
échouaient en timeout. Deux causes racines (debug par agent, vérifié indépendamment) :

1. **Port DNS perdu** (tom-relay/dns.rs) : `NameServerConfig::new` 0.26 prend une
   IP, plus un SocketAddr — la migration avait silencieusement perdu le port,
   le resolver visait le port 53 au lieu du port aléatoire du serveur de test.
   Fix : `ConnectionConfig::udp()/tcp()` + `conn_cfg.port = addr.port()`.
2. **Métadonnées de réponse incomplètes** (tom-connect/test_utils) : en 0.26,
   muter seulement `message_type = Response` ne suffit plus — flags (AA/TC/RA/AD)
   non initialisés → réponse rejetée par le resolver.
   Fix : `Metadata::response_from_request(&packet.metadata)`.

### swarm-discovery 0.5 → 0.6.1 (H.4)

Après le bump hickory, `cargo deny` échouait toujours : swarm-discovery 0.5.0
(mDNS, feature address-lookup-mdns de tom-connect) tirait encore hickory-proto
0.25.2. swarm-discovery 0.6.1 dépend de hickory-proto ^0.26 — bump sans
adaptation de code (API mdns.rs inchangée), vérifié par
`cargo test -p tom-connect --lib --features address-lookup-mdns` (83 ok).

### Leçon réappliquée — lock FFI

Le `Cargo.lock` de tom-protocol-ffi (hors workspace, invisible pour cargo-deny,
mais embarqué dans le XCFramework) a été resynchronisé : hickory unifié 0.26.1.
Même classe de problème que les fails FFI de la PR #43 et le webpki vulnérable
de la PR #42.

## Validation

- `cargo clippy --workspace -- -D warnings` ✅
- `cargo test -p tom-connect --lib --features address-lookup-mdns` : 83 ok ✅
- `cargo test -p tom-relay --lib` : 9 ok ✅
- `cargo deny check advisories bans sources` : **advisories ok** (ignores hickory retirés) ✅
- Gate finale workspace : voir PR

## Bilan advisories après ce chantier

Restent dans deny.toml, justifiées : rand (fix 0.9.3 non publié, exposition
faible), paste + atomic-polyfill (unmaintained transitifs), lru via ratatui
(IterMut non utilisé). **Toutes les advisories corrigeables sont corrigées.**
