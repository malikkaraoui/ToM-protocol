---
verdict: GO
tip: ebca4d89c7f58c4f9524c12bf98ca6a9b8aaac90
niveau: N2
---

# R015 — doublage de M0016 (genèse et vision de ToM)

- **Objet** : `vault/decisions/2026-10-18-GENESE-vision.md` (+ `genese/00-inventaire.md`, `genese/10-extraits.md`), branche `docs/genese-vision-2026-10`, PR #77.
- **Doubleur** : contradicteur Opus frais, lecture seule, sans accès au raisonnement de l'auteur. Aucun fichier écrit par lui ; ce rapport est retranscrit par l'auteur à partir de ses deux réponses.
- **Base du code relu** : `aaeee84` (le diff `aaeee84..ebca4d8` ne touche que le vault).

## Tour 1 — commit `8e6f622` : RESERVE

- **Échantillon** : 58 citations tirées sur ~180 (32 %), réparties sur sources du fondateur, fondations, plans, code et git. Aucune citation fausse. Décalages signalés sur v0.1 l.81-82, POP l.29 et `state.rs:1880/1906` ; les deux premiers et le dernier ont été retirés au tour 2 (recompte du doubleur), seul POP l.29 → l.4 était fondé.
- **Constats forts du code, tous confirmés** : (a) `replicate_to` sans appelant hors tests dans tout `crates/` ; (b) `record_bytes_relayed` idem ; (c) une annonce de rôle signée d'un pair inconnu l'inscrit `Online` (`state.rs:951-957`, arrivée par gossip) ; (d) « imprévisible » présent dès `2cbf495` — mais MISSION l.4 date le premier jet du 2026-06-26 : « datation ambiguë » plutôt qu'« erreur ».
- **Réserve bloquante 1** : thème 12 noté ✅ avec « écart code : néant », en s'appuyant sur CLAUDE.md limite #8. Le code le contredit : `crates/tom-protocol-ffi/src/types.rs:489-560`, `lib.rs:686-690`, `lib.rs:1305`, `TomModels.swift:428`, `TomNodeService.swift:300-307`.
- **Verdicts ✅ attaqués** : thème 1 tient (tracer le double sens de « blockchain ») ; thème 8 tient sur la vision, écart code incomplet (relais n0 par défaut) ; thèmes 10 et 14 devraient être ⚠️.
- **Remarques** : règle « contrainte de plateforme » attribuée au fondateur alors qu'elle vient du mandat ; réserve « non confirmé » perdue sur `vault/40-roadmap.md` l.123 ; extension de « un seul moteur » à marquer [HYPOTHÈSE] ; charte l.90-91 ne parle pas d'attaquant proportionnel.

## Correction — commit `ebca4d8` (une seule)

Thème 12 passé à ⚠️, écart code ⛔ cité, piste réécrite (filtrage à la frontière du SDK), issue #78 ouverte pour CLAUDE.md limite #8. Remarques intégrées : thèmes 10 et 14 en ⚠️, relais n0 par défaut au thème 8 (`tom-connect/src/defaults.rs:22-35`, ADN l.31), réserve de la roadmap rétablie, règle de plateforme attribuée au mandat (F01 l.32), « un seul moteur » étendu marqué [HYPOTHÈSE], « blockchain » tracé au thème 1, datation MISSION l.41 requalifiée, annexe complétée (+9 vérifications, total 189).

## Tour 2 — commit `ebca4d8` : GO

- Réserve 1 **levée** ; 16 nouvelles citations contrôlées, toutes exactes à un décalage d'une ligne près.
- Aucune réserve bloquante restante.
- **Remarques non bloquantes** (non reprises, pour ne pas ouvrir un nouveau tour) : plage `TomNodeService.swift:300-306` à étendre à l.307 ; `TomModels.swift:428` est le libellé, le décodage passe par Codable ; citer `TomNodeService.swift:160,164` et `tom-transport/src/node.rs:273-275` pour « le défaut de l'app » au thème 8.
