# P1 — Namespace de rendez-vous de test (isolement des bancs)

> Design-first (2026-07-21). Prérequis P1 du banc « rôles sous charge »
> (`banc-roles-sous-charge.md` §5) : sans lui, tout scénario R4 et tout capstone
> risque de re-polluer le carnet de rendez-vous RÉEL (incident 20/07 : ~150
> fantômes). Périmètre vérifié dans le code : `RENDEZVOUS_NAMESPACE` const
> (`tom-dht/src/lib.rs:40`), consommée par `rendezvous_slot_key()` ; deux
> fonctions publiques (`rendezvous_publish`/`rendezvous_discover`) ; UN SEUL
> appelant externe (`runtime/loop.rs:1316,1319`).

## §1 Décision

**API-first, préfixe forcé, zéro variable d'environnement.**

```rust
// tom-dht — type opaque
pub struct RendezvousNamespace(Vec<u8>);
impl RendezvousNamespace {
    pub fn production() -> Self;            // b"tom-protocol-rendezvous-v1" (INVARIANT WIRE)
    pub fn test(label: &str) -> Self;       // b"tom-test-" + label assaini ([a-z0-9-], ≤64)
    pub fn is_production(&self) -> bool;
}
```

- `rendezvous_slot_key(ns, i)`, `rendezvous_publish(dht, ns, addr)`,
  `rendezvous_discover(dht, ns, own_id)` prennent le namespace en paramètre
  (breaking interne assumé : 1 appelant + tests tom-dht).
- `RuntimeConfig.rendezvous_namespace: Option<String>` — `None` (défaut) =
  production ; `Some(label)` = `RendezvousNamespace::test(label)`.
- Au boot avec un namespace non-prod : **WARN explicite**
  (« rendez-vous NON-PRODUCTION "tom-test-…" — ce nœud est INVISIBLE du réseau
  public »). Jamais silencieux.
- **Les apps (Swift/FFI) n'exposent RIEN** — seul un code Rust qui construit
  explicitement la RuntimeConfig peut l'activer (bancs tom-stress). Le flag CLI
  arrivera avec le scénario R4-F, pas avant.

## §2 Pourquoi PAS une variable d'environnement

L'option `TOM_RENDEZVOUS_NAMESPACE` (esquissée dans la revue DOCTRINE du 20/07)
est **rejetée** : une env var est un **vecteur d'injection** — quiconque
contrôle l'environnement d'un process (wrapper, systemd drop-in, CI, parent
compromis) isolerait un nœud légitime du réseau **sans toucher au binaire ni à
la config**, et en silence (le nœud « marche », il ne trouve juste personne).
La fragmentation silencieuse est exactement la classe d'incident qu'on veut
rendre impossible, pas facile.

## §3 Red-team du mésusage

| # | Attaque / mésusage | Parade |
|---|---|---|
| 1 | Fragmentation silencieuse d'une app légitime (opérateur/env) | Pas d'env var ; activation = code explicite ; WARN au boot ; apps jamais câblées |
| 2 | Label forgé pour rejoindre la prod (`test("tom-protocol-rendezvous-v1")`) | **Préfixe forcé** `tom-test-` ⇒ ≠ prod par construction, testé |
| 3 | Squat/pollution d'un namespace de test public (fantômes tiers) | Même défense que la prod : entrées signées, `rendezvous_entry_authentic` (`loop.rs`) rejette le non-authentique ; bruit résiduel assumé (identique prod) |
| 4 | Éclipse d'une victime en la forçant sur un namespace test | Exige de contrôler sa RuntimeConfig = compromission locale déjà totale ; **aucun nouveau vecteur** (l'env var l'aurait créé) |
| 5 | Charge parasite sur la DHT Mainline (namespaces test) | 8 clés BEP-0044 par banc, TTL DHT naturel — négligeable, assumé |
| 6 | Un banc test croit parler à la flotte (faux vert) | C'est le but inverse : l'isolement est structurel ; le banc R4 vérifie `source_amorcage=rendezvous` + entrées signées |

## §4 Tests (verrous)

1. **Golden wire** : `production()` == `b"tom-protocol-rendezvous-v1"` octet
   pour octet — l'invariant du réseau réel ne PEUT pas dériver.
2. Préfixe forcé : tout `test(label)` commence par `tom-test-` ET ≠ production,
   y compris `label = "tom-protocol-rendezvous-v1"`.
3. Isolation des clés : `rendezvous_slot_key(prod, i) ≠ slot_key(test, i)`
   pour chaque slot i (0..8).
4. Sanitisation : label hostile (`"../;π€ FOO"`) → assaini `[a-z0-9-]`, ≤ 64.
5. Runtime : `RuntimeConfig { rendezvous_namespace: Some(...) }` → le WARN est
   émis et le namespace passé aux deux appels de `loop.rs`.

## §5 Hors périmètre P1 (notés, pas oubliés)

- Flag CLI `--rendezvous-namespace` de tom-stress → livré avec R4-F.
- Exposition du namespace dans le status :9091 (observabilité) → avec R4-F.
- Rotation du carnet (écart #2 du prisme) → chantier séparé, non commencé.
