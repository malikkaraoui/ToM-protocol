---
verdict: GO
tip: b9944b1b84c8e1cfa633edceec1c522946bf84aa
niveau: N2
---

# R016 — doublage de M0017 (genèse et vision de ToM, V2)

- **Objet** : `vault/decisions/2026-10-18-GENESE-vision.md` (version 2), branche `docs/genese-vision-v2`, PR #80.
- **Doubleurs** : deux contradicteurs Opus frais, lecture seule, sans accès au raisonnement de l'auteur (un par tour). Aucun fichier écrit par eux ; ce rapport est retranscrit par l'auteur à partir de leurs réponses.
- **Base** : `be8e18e` (le diff `aaeee84..be8e18e` ne touche que le vault).
- **Consigne durcie** (mandat M0017) : 100 % des phrases de cause et des dates de jalon, 20 % des autres citations, les 25 défauts du mandat un par un, attaque de chaque recommandation F1-F9 au regard de la philosophie du fondateur.

## Tour 1 — commit `ad1ae70` : RESERVE (7 réserves bloquantes)

- **Comptes** : 53 phrases de cause auditées (46 justes) ; 68 dates de jalon (67 justes) ; 57 citations [VÉRIFIÉ] tirées (une sur cinq des 286, dans l'ordre d'apparition) : 57 justes.
- **25 défauts du mandat** : 22 corrigés, 3 partiels (n°1, n°8, n°25 — voir réserves 1, 2, 7).
- **Réserves** (toutes recontrôlées à la source par l'auteur, toutes fondées) :
  1. §2 : « No need for JSON wire format compatibility » (kickoff l.21) justifie le refus de coexistence TS, pas le choix de Rust ; raison écrite du choix : article l.954.
  2. §1 : « Au départ (janvier) » faux pour v0.1 (PDF du 2025-09-28).
  3. §13/§1 : la valeur passe en L2 le 05/07 (`bcd8978`), avant la revue Fable du 06/07 ; le conditionnel date du 16/07 ; citation en MASTER-MAP l.188.
  4. §8 : « protège l'universalité » étiqueté [VÉRIFIÉ] sans source.
  5. F4 : recommander de fermer toute valeur convertible contredit MISSION l.285 et F5.
  6. §9/F6 : MISSION l.47 est une cible (§1, « Non négociable », l.13), pas un état à réécrire d'après le code.
  7. §2/F9 : une interface C publique existe déjà (`tom_protocol_ffi.h`, cbindgen, 24 `extern "C"`) ; la prémisse « créer une ABI C » était fausse.
- **Attaque des recommandations** (prise en compte) : F1 — trou Sybil commun à C et D (qui garantit que les signataires sont les vrais voisins ?) ; F2 — MISSION « jusqu'à 7 » est plus récente que la CARTE ; F3 — EXP-4 ne porte que sur le tirage de la graine ; F5 — le ré-ancrage peut devenir du « ledger creep » et rouvrir le mur n°4 ; F6 — gardiens 24/24 = risque de concentration (ticket d'entrée + tirage égal) ; F7 — la surveillance de wp-v1 repose sur des métadonnées ; F8 — l'option C est un changement de vision.
- **Remarques non bloquantes corrigées au passage** : causes de §4 (ADR-011 vs quorum), lecture du recadrage du 20/07, PoC iroh « teste » le remplacement, heure UTC du PDF Step2, lignes Step2 §3 / anti-ravivage / MASTER-MAP, portée de « contraire à la vision » (push et réveil permanent), brief « valeur spéculative ou convertible », paliers du brief, compte de points de l'annexe.

## Tour 2 — commit `b9944b1` : GO

- Les 7 réserves sont levées (ligne et preuve source pour chacune).
- 44 points du diff `ad1ae70..b9944b1` contrôlés (dates, citations ajoutées mot à mot, causes ajoutées) : aucune réserve. Aucune IP ni nom d'hôte.
- **8 remarques non bloquantes** (une citation « pas de jeton-actif natif » inexacte, renvois de lignes, une phrase sans source) : ouvertes en **issue #81**, conformément à la règle « une remarque devient une issue, jamais un nouveau tour ».

## Verdict

**GO** sur `b9944b1b84c8e1cfa633edceec1c522946bf84aa`.
