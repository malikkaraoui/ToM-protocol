# Conception — Nom d'usage via le rendez-vous DHT (chantier #27)

> Statut : **conception (doc-avant-code)** — feature protocolaire touchant un enregistrement DHT **signé**, donc red-team de l'abus AVANT toute ligne de code (règle projet). Rédigé 2026-07-15.

## Problème (observé terrain)

En cellulaire (pas de mDNS, pas de payload gossip portant le username), un pair découvert via le rendez-vous DHT n'apparaît que par son hex court (« 80eb9196 ») au lieu de son nom (« iPhone »). `DhtNodeAddr` (`crates/tom-dht/src/lib.rs:66`) transporte `node_id`, `relay_urls`, `direct_addrs`, `timestamp`, `sig` — **mais pas de username**. Le nom ne se propage donc que par les canaux LAN.

## Proposition

Ajouter un champ **optionnel, borné, signé** :

```rust
/// Nom d'usage non-autoritaire (hint d'affichage). Borné, assaini. Lié au
/// node_id par la signature → un nœud ne peut nommer que LUI-MÊME.
#[serde(default)]
pub username: String,   // "" = absent
```

- **Inclus dans `signing_bytes()`** (après `direct_addrs`, avec séparateur NUL) → authentifié par la clé du `node_id`. Un enregistrement dont le username est falsifié casse la signature et est rejeté (`rendezvous_entry_authentic`, `loop.rs:862`).
- `#[serde(default)]` → rétro-compat : un vieux nœud ignore le champ ; un nouveau nœud lisant un vieux record voit `""`.

## Red-team (obligatoire — enregistrement signé)

| Vecteur | Analyse | Mitigation |
|---|---|---|
| **Usurpation** (« je m'appelle iPhone pour me faire passer pour le tien ») | La signature lie username ↔ node_id. Un attaquant ne peut mettre un username que sur SON PROPRE node_id. Il peut afficher « iPhone » mais son node_id diffère. | **L'ancre de confiance reste le node_id (hex court), TOUJOURS affiché à côté.** Le username est un hint. **Jamais** router/dédupliquer/autoriser par username. |
| **Squatting** (deux nœuds « iPhone ») | Autorisé — c'est un hint d'affichage, le node_id désambiguïse. | Aucune. Ne pas traiter comme unique. |
| **Injection** (username = `\n`, contrôle, non-UTF8 → pollue logs/collecteur UDP JSON) | Un username malveillant pourrait injecter dans le Live Log / collecteur. | **Assainir à la lecture ET à l'écriture** : `is_control()` retirés, longueur ≤ 32 octets, UTF-8 valide sinon `""`. |
| **DoS taille** (username géant gonfle le record DHT / BEP-0044 ~1000 octets) | Un record trop gros est rejeté par le mainline DHT. | Plafond dur 32 octets appliqué avant publication ET avant acceptation. |

## Portée (cross-crate — nécessite pipeline complet + gate)

1. `crates/tom-dht/src/lib.rs` : champ `username`, l'inclure dans `signing_bytes()`, helper de sanitation.
2. `crates/tom-protocol/src/runtime/loop.rs` : peupler depuis `config.username` à la publication (`build_self_dht_addr` / `spawn_rendezvous_round`) ; à la lecture, propager vers le nom d'affichage de la topology (comme les autres canaux).
3. FFI/Swift : probablement **rien** — le nom d'affichage est déjà résolu via le mapping central (`displayName(for:)`) ; vérifier que la topology reçoit bien le username issu du DHT.
4. Bump build.

## Tests obligatoires

- serde round-trip **avec et sans** username (rétro-compat `#[serde(default)]`).
- signature **couvre** username : tamper du username après signature → `verify` rejette.
- sanitation : longueur > 32 → tronqué/rejeté ; caractères de contrôle retirés ; non-UTF8 → `""`.
- intégration : un record DHT signé avec username « iPhone » lu par un pair → topology affiche « iPhone » + node_id.

## Validation

Cross-crate tom-dht → tom-protocol : `cargo test -p tom-dht -p tom-protocol && cargo clippy --workspace -- -D warnings && cargo test --workspace`. FFI hors-workspace → `bash scripts/check-ffi.sh`. Propagation réelle (nom qui suit en cellulaire) → test flotte avec NAS up.
