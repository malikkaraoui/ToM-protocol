# Chantier S1 — API SDK Rust (façade tom-sdk) · Suivi d'exécution

> Démarré : 2026-06-10 22:18 · Référence : `2026-06-10-roadmap-sdk.md` (Phase S1) · Précédent : chantier S0 clôturé (`2026-06-10-chantier-s0-suivi.md`)
> Règle : un commit atomique par tâche · gate clippy+test workspace avant clôture · pas de push (dette handoff §25)
> Décisions appliquées : D1 (tag git), D5 (façade `tom-sdk` fine au-dessus de tom-protocol)

## Tableau de bord

| Tâche | Description | Statut | Commit |
|---|---|---|---|
| S1.0 | Crate `tom-sdk` : squelette + TomClient haut niveau | ✅ | `0fd02db` |
| S1.1 | Étanchéité : aucun type forké dans l'API tom-sdk (🔁 par construction, voir journal) | ✅ | `0fd02db` |
| S1.2 | Erreurs : TomSdkError documentée + #[non_exhaustive] | ✅ | `0fd02db` |
| S1.3 | Docs : deny(missing_docs), examples/ ×3, README crate, métadonnées Cargo | ✅ | `5958ab5` |
| S1.4 | Corrections tom-protocol (🔁 faux positifs d'audit, voir journal) | ✅ | aucun |
| S1.V | Validation : exemples exécutés en réel + gate workspace | ✅ | (docs) |

## Journal de chantier

### 2026-06-10 22:18 — Ouverture

Principe directeur (D5) : `tom-protocol` reste le moteur, `tom-sdk` est le contrat public.
La façade ne re-exporte NI `RuntimeCommand`, NI les types des crates forkés (tom-connect/tom-transport).
Conception de l'API basée sur l'usage réel de tom-tui (consommateur de référence identifié à l'audit).

### 2026-06-10 22:30 — Intermède : commit push demandé (« au passage »)

6 commits groupés du travail en attente + push `2903fcf..bb0846b` (14 commits au total avec S0) :

| Commit | Contenu |
|---|---|
| `6c5ef94` | feat(apple) : StatusServer iOS/tvOS + cible macOS réparée |
| `401c68d` | feat(infra-web) : dashboard multi-nœuds |
| `b68cac9` | feat(tom-tui) : statut http enrichi (schema v1) |
| `4c777ca` | chore(claude) : config atelier modulaire |
| `5374883` | docs(plans) : plan dashboard + journal S1 |
| `bb0846b` | chore(git) : untrack graphify-out (~700k lignes de bruit) |

Gates passées avant push : clippy workspace ✅ · cargo test workspace ✅ (exception multi_node documentée) · pnpm test 771/771 ✅ · smoke observability pre-push ✅.

**Notes d'incidents** :
- Mon amend S0.2 (`57b268e`) avait embarqué par accident les 3 Assets macOS stagés — contenu identique à l'intention, pas de réécriture d'historique, noté dans le message de `6c5ef94`.
- Le hook `guard-tests-before-push.sh` ne reconnaît pas `cargo test` (patterns npm/jest/pytest/mvn/gradle uniquement) → marqueur posé via `pnpm test` (légitime : du TS était modifié). **Backlog : ajouter `cargo test` aux patterns du hook** (MAJ test/hooks.js + hooks-manifest.json requises, cf. §11).

### 🔐 Findings sécurité (review auto post-commit) — 4 HIGH sur StatusServer.swift

La review automatique signale sur `StatusServer.swift` (iOS + tvOS) :
1-2. **Exposition réseau non authentifiée** (le snapshot répond à tout client TCP, pas de restriction d'interface) ;
3-4. **CORS wildcard** (`Access-Control-Allow-Origin: *`) → exfiltration cross-origin / DNS rebinding possible.

**Analyse contextuelle** : l'exposition LAN est le but de la feature (dashboard dev multi-appareils — le Mac interroge iPhone/Apple TV ; un bind loopback la casserait). Données exposées : node_id (adresse réseau publique par design), pairs, relay actif — sensibilité modérée, surface = LAN du dev.
**Tradeoff assumé pour l'outillage dev, MAIS durcissement requis avant tout build de distribution** :
- [ ] gater `startStatusServer()` derrière `#if DEBUG` (ou flag de launch),
- [ ] valider le header `Host:` contre une allowlist (anti DNS-rebinding),
- [ ] remplacer le CORS `*` par l'origine du dashboard,
- [ ] option : bearer token par session loggé au démarrage.
→ **Décision finale : Malik.** Ajouté au backlog priorisé.

### S1.0-S1.2 ✅ — commit `0fd02db` : le crate `tom-sdk` existe

**Architecture livrée** (`crates/tom-sdk`, membre du workspace) :
- `TomClientBuilder` : username, encryption, relay_url(&str), n0_discovery, dht, identity_path, data_dir → `connect()`.
- `TomClient` : send/send_text, read receipts, groupes complets (create/invite/accept/decline/leave/kick/update_role), connected_peers, metrics, shutdown.
- **Flux d'événements unifié** : les 3 canaux du runtime (messages, status_changes, events) sont fusionnés par une tâche tokio en un seul `client.next_event() → Event` — l'application n'écrit qu'une boucle.
- **Étanchéité par construction** (S1.1 résolu différemment de la roadmap) : plutôt que des newtypes wrappers dans tom-protocol, la façade ne re-exporte AUCUN type forké — `RelayUrl → String`, `PathEvent → {peer, direct, rtt_ms}`, `EndpointAddr → ticket JSON opaque` (`ticket()`/`add_peer_ticket()`, pattern déjà validé par le FFI). Les re-exports (NodeId, Group*, MessageStatus, MetricsSnapshot) sont tous des types ToM originaux.
- `#![deny(missing_docs)]`, `TomSdkError` et `Event` `#[non_exhaustive]`, doc-tests compilés.
- **Test d'intégration** : 2 nœuds échangent un message chiffré+signé via tickets, sans infra, en 0,15 s.
- Événements internes volontairement non exposés (Forwarded, Backup*, Role*, Subnet*, gossip, antispam, relay embarqué) — documenté dans le rustdoc : usage avancé → tom-protocol direct.

### S1.3 ✅ — commit `5958ab5`

- 3 exemples : `01_send_message`, `02_group_chat`, `03_own_relay`. **01 et 02 exécutés en réel** : message E2E (chiffré: true, signature: true) et flux groupe complet (invitation → join → fan-out) fonctionnels.
- README crate : install par tag git (D1), quickstart 10 lignes, tableau concepts, garanties protocole.
- Métadonnées Cargo complètes (description, license MIT, repository, keywords, categories).

### S1.4 ✅ — aucun commit : les findings d'audit étaient faux ou assumés

- Les `unwrap()` de `runtime/metrics.rs:205,229` et `bootstrap.rs:142` signalés par l'audit sont **dans des `#[cfg(test)]`** — pas de chemin de production, rien à corriger. (Leçon : l'audit d'agent n'avait pas vérifié le contexte module test.)
- Les 3 `allow(dead_code)` (`group/manager.rs:37`, `bootstrap.rs:6,38`) sont des placeholders **délibérés, commentés, pour phases futures** (RelayAssist/DhtAssist, Manual, candidate_id) — conservés tels quels.

### S1.V ✅ — clôture 2026-06-10 23:0x

- `cargo clippy --workspace -- -D warnings` ✅ (tom-sdk inclus).
- `cargo test --workspace --exclude tom-integration-tests` ✅ 0 échec (même exception multi_node que S0 : NAS offline).
- Critère roadmap « un projet externe compile les exemples » : validé en interne (exemples = consommateurs purs de l'API publique) ; la consommation par tag git sera vérifiable au prochain tag.

## Clôture S1 — bilan

**La promesse PRD « intégration en quelques lignes » est tenue** : 10 lignes pour un nœud ToM fonctionnel. Chemin critique restant vers le SDK public : **Phase S2** (Swift Package + release XCFramework) et **S3** (spec protocole), cf. roadmap.

### Backlog généré
1. Tag `v0.3.0` après merge (rend l'install par git effective, cf. README tom-sdk).
2. Exposer le relay embarqué dans le builder (v0.2 du SDK) si demande.
3. Exemple 03 (own_relay) non testé en réel — NAS offline ; à valider au retour de l'infra.

<!-- Entrées ajoutées au fil de l'exécution -->
