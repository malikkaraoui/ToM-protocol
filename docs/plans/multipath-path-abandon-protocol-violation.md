# Proptest multipath : PROTOCOL_VIOLATION sur PATH_ABANDON non réciproqué

> Doc de raisonnement (design-before-code) — 2026-07-17. Tranche le proptest
> `random_interaction_with_multipath_simple_routing` qui bloquait la pre-push gate.

## Symptôme

`cargo test -p tom-quinn-proto random_interaction_with_multipath_simple_routing` échoue
de façon déterministe (rejeu du seed `59a84bb8…` de `proptest-regressions/tests/proptest.txt`).
Assertion cassée : `proptest.rs:118` — côté **client**, `poll_to_close` remonte
`TransportError(PROTOCOL_VIOLATION, "peer failed to respond with PATH_ABANDON in time")`,
que `allowed_error()` refuse à juste titre.

Cas minimal (shrink) :

```text
OpenPath(Client, Available, 1)   → path 1 vers 2.2.2.1
OpenPath(Client, Available, 1)   → path 2 vers 2.2.2.1
Drive(Client)
ClosePath(Client, 1, 0)          → le client abandonne path 1
PathSetStatus(Server, 0, Backup)
PassiveMigration(Server, 0)      → l'adresse source du serveur change (rebinding NAT simulé)
```

## Chaîne de preuve (trace complète : `regression_backup_passive_migration` + RUST_LOG=trace)

1. Le client abandonne path 1 et **arme une deadline de 3×PTO à l'émission** du premier
   PATH_ABANDON (`packet_builder`, `AbandonState::ExpectingPathAbandon`). Les retransmissions
   ne prolongent PAS la deadline (commentaire explicite dans le code).
2. La migration passive casse la route : les PATH_ABANDON (pn 11, 15, 25, 30…) partent vers
   l'ancienne adresse du serveur → **tous perdus**. Comptage des frames effectivement reçues
   par le serveur sur toute la trace : 4 PATH_ACK_ECN, 2 PATH_RESPONSE, 1 CRYPTO —
   **zéro PATH_ABANDON**. Le serveur n'a jamais reçu le signal.
3. Le serveur, qui de son point de vue est encore en train d'OUVRIR path 1, continue
   légitimement d'y sonder (PATH_CHALLENGE) ; il finira par abandonner path 1 de lui-même
   sur timeout `PathOpen` — trop tard.
4. Le client reçoit une de ces sondes après sa deadline →
   `on_packet_authenticated` → `move_to_closed(PROTOCOL_VIOLATION("peer failed to respond
   with PATH_ABANDON in time"))` → **la connexion entière est tuée** pour un chemin déjà mort.

## Verdict : (A) vrai bug — c'est l'enforcement lui-même qui est fautif

- Punir le silence est **indécidable** : impossible de distinguer un pair non conforme d'un
  réseau qui a perdu notre propre signal. Ici le pair est irréprochable ; c'est notre frame
  qui n'est jamais arrivée. Fermer TOUTE la connexion (tous chemins) est disproportionné et
  casse exactement le scénario que multipath doit sauver (migration + chemin dégradé).
- Upstream n0-computer/quinn a tranché **à l'identique** : commit `2903b55dd` (PR #436,
  2026-02-19) « fix(proto): Avoid generating protocol violation errors in bad network
  conditions » — même scénario (leur doc-comment décrit une migration passive qui condamne le
  chemin porteur du PATH_ABANDON), même conclusion : « It's generally hard/impossible(?) to
  decide whether a PATH_ABANDON frame not arriving means the client is not protocol compliant
  or just under bad network. »
- Donc PAS (B) : élargir `allowed_error()` aurait masqué un défaut de conception reconnu et
  corrigé upstream. L'assertion du proptest faisait son travail.

## Fix : port upstream #436 EN ENTIER (règle : jamais de demi-port)

| # | Upstream (quinn-proto) | Notre arbre (tom-quinn-proto) | Nature |
|---|------------------------|-------------------------------|--------|
| 1 | `close_path` @740 | `mod.rs:754` | Supprimer le `set_max_path_id(+1)` immédiat à l'abandon local |
| 2 | `on_packet_authenticated` @3450 | `mod.rs:3404-3422` | Supprimer tout le bloc `ExpectingPathAbandon` → PROTOCOL_VIOLATION |
| 3 | réception `Frame::PathAbandon` @5037 | `mod.rs:4965-4981` | `abandon_state` → bool `draining` (`mem::replace`) ; **c'est ICI que `set_max_path_id(+1)` est désormais accordé** |
| 4 | `packet_builder` PATH_ABANDON @5847 | `mod.rs:5632-5674` | Supprimer l'armement deadline + DiscardPath `3×send_pto+3×abandoned_pto` + warn « after path was already discarded » |
| 5 | `paths.rs` | `paths.rs:217, 229-243, 300, 341` | Champ `abandon_state: AbandonState` → `draining: bool` ; enum `AbandonState` supprimé |
| 6 | `tests/util.rs` @182/@214 | `util.rs:185/217` | « packet … lost » → « no route … for packet » (avec routing table, absence de route ≠ perte aléatoire) |
| 7 | `tests/proptest.rs` | idem | + `regression_peer_ignored_path_abandon` (adapté : nos TestOp sont des tuple-variants) ; on garde AUSSI `regression_backup_passive_migration` (notre cas shrunk) et le seed dans `proptest-regressions/` |

Adaptations assumées (documentées, pas des demi-ports) :
- Notre base est plus ancienne : la chaîne PROTOCOL_VIOLATION est chez nous un `&'static str`
  (upstream l'avait déjà passée en `format!` avec path_id) — on supprime le bloc, delta nul.
- `util.rs:185` est dans `drive_client` (direction client→serveur) : upstream y a laissé un
  libellé copy-paste « server to client » ; on corrige l'étiquette de direction chez nous.
- On ne porte QUE #436. Les refactors intermédiaires (#427, #430, #432, #433) et postérieurs
  (#438 timers de loss detection à l'abandon, #443, #449, #452, #458) ne sont PAS portés —
  à réévaluer à froid, EN ENTIER, sous canari (même politique que #4296, cf. handoff 17/07).

## Conséquences sémantiques à assumer (choix upstream, repris)

- **Compensation anti-DoS** : le crédit `MAX_PATH_ID` n'est plus accordé à l'abandon local
  mais **à la réciprocité** (réception du PATH_ABANDON du pair). Un pair malveillant qui ne
  réciproque jamais n'obtient pas de crédit de chemins supplémentaires → pas d'accumulation
  mémoire pilotable.
- Un chemin abandonné dont le pair ne réciproque jamais **reste dans la map jusqu'à la fin
  de la connexion** (plus de DiscardPath armé à l'émission) : occupation stable, pas une
  fuite — bornée par `max_concurrent_multipath_paths` et par le non-octroi de crédit
  ci-dessus. Assumé upstream, assumé ici.
- Exhaustivité vérifiée : 11 occurrences de `AbandonState` dans le crate, toutes couvertes
  par les hunks (grep complet avant patch).

## Validation

1. `regression_backup_passive_migration` (repro dédiée, trace) : rouge avant → verte après.
2. Rejeu automatique du seed (`proptest-regressions/tests/proptest.txt` conservé).
3. `regression_peer_ignored_path_abandon` (scénario upstream adapté) : vert.
4. `cargo test -p tom-quinn-proto` complet + `cargo clippy --workspace -- -D warnings` + pre-push gate.
