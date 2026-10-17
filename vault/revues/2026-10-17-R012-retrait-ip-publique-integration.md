---
verdict: GO
tip: f679c1a3418ea6b791cff4f1caaed1c1b335ca31
niveau: N2
---

# R012 — Intégration de `fix/retrait-ip-publique` après le merge de `main` (doublage N2 court)

Mandat F03/M0013, partie B. Cette revue couvre seulement le delta `3ef96ea..f679c1a`.
Le contenu de la branche a été relu dans **R009 (GO sur `3ef96ea`)**, voir
`vault/revues/2026-10-17-R009-retrait-ip-publique.md`. Relecture par un sous-agent frais,
en lecture seule, sans build ni test. Aucune valeur d'IP n'est affichée, seulement des comptes.

## Merge

- `f679c1a` = `Merge branch 'main' into fix/retrait-ip-publique`. Parents : `3ef96ea` et
  `13ec2b8` (= `origin/main`, qui contient le correctif #63 / R011). Pas de rebase.
- Delta `3ef96ea..f679c1a` : 10 fichiers, +712/-28, tous venus de `main` (`node.rs` #63,
  `docs/MISSION.md`, archives, revues et reprises du vault).
- Contrôle 3-voies sur 88 fichiers (base `99aa090`) : 10 viennent de `main`, 78 de la branche,
  0 résolution manuelle, 0 conflit.

## Garde et objets git (comptes seulement)

```
scripts/verifier-ip-interdites.sh (disque, tip) : OK (aucune IP interdite), rc=0
arbre f679c1a (1562 blobs)            : noms_de_fichiers=0 contenus_hors_exemptions=0 contenus_exemptes=7
blobs introduits 3ef96ea..f679c1a (12) : noms_de_fichiers=0 contenus_hors_exemptions=0
arbre 3ef96ea (1557 blobs, rappel R009) : noms_de_fichiers=0 contenus_hors_exemptions=0 contenus_exemptes=7
```

Seul chemin exempté : `vault/30-discoveries.md` (7 lignes), un fichier intouchable déjà connu dans R009.
Le scanner d'objets reprend la logique de la garde (mêmes regex, même canonicalisation, même
SHA-256) et lit les objets git (`ls-tree`, `cat-file`, `rev-list --objects`), pas le disque.
Il couvre donc l'angle mort #67. Les 10 fichiers venus de `main` contiennent 0 ligne interdite.

## Historique de la branche (constat de la session mère, au merge vers `main`)

Les blobs nouveaux pour `main` (`origin/main..` merge, 92 blobs) comptent 7 lignes interdites,
réparties dans 5 fichiers (`docs/plans/r14-ipv6-first-class.md`,
`experiments/iroh-poc/results/00-baseline-home-wifi.jsonl`, `…/01-commute-fr-ch3_ecole.jsonl`,
`vault/10-mailbox.md`, `vault/40-roadmap.md`). Ce sont des versions intermédiaires entre les
deux commits de retrait `0b24450` (IPv4) et `ec8b180` (IPv6), qui portent tous deux la mention
« historique non réécrit ». Elles n'introduisent aucune valeur nouvelle : leurs empreintes sont
toutes incluses dans la version d'origine de ces fichiers (`0b24450^`, ancêtre de `origin/main`).
Les deux commits sont déjà publics via `origin/fix/retrait-ip-publique`. Le merge rend ces
valeurs accessibles depuis `main`, mais n'expose rien de plus. Cette limite est connue (en-tête
de la garde, #67). Arbre du tip après merge : 0 hors exemptions.

## Verdict

GO. Le merge est propre et n'apporte aucune IP interdite.
