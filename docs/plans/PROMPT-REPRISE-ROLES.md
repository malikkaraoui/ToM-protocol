# PROMPT DE REPRISE — le réseau est un ORGANISME À RÔLES, pas un tuyau à messages

> Session précédente : 2026-07-20. Rédigé après recadrage fort de Malik.
> **À lire EN PREMIER, avant toute action, avec QMD (`mcp__qmd__*`), en prenant le temps.**

## 0. LA CRITIQUE FONDATRICE DE MALIK (le cœur — ne pas la perdre)

**Texte exact de Malik (20/07, verbatim — à garder à l'esprit en permanence)** :

> « je ne veux pas simplement que tu simules des nœuds et envoie des messages. Le réseau a
> des relais, des backup et d'autres rôles. […] Aujourd'hui tu raisonnes comme : le réseau
> est censé envoyer un message, donc bah t'envoies des messages et s'il les réceptionne et
> s'il y a accusé de réception c'est bon il a fait le travail. Le truc c'est que le réseau
> ne s'arrête pas là. En fait c'est pas son seul travail et c'est pas son but primaire. Je
> te rappelle qu'on a une L1, on a des sous-réseaux, on a des rôles — différents rôles, on
> a la validation, on a les observateurs, on a les relais, on a les backup, on a les
> personnes qui détiennent le carnet de rendez-vous pour mettre deux personnes qui ne se
> connaissent pas en lien, un peu comme les serveurs DNS fixes, sauf que là c'est des rôles
> qui TOURNENT. Tout ça, je ne le vois pas assez, ça ne transpire pas assez dans ton texte,
> donc je te prierai de relire l'intégralité des notes. »

> « Ce que j'ai travaillé il y a quelques mois — j'ai pris des notes, j'ai fait des
> documents — c'est pas pour rien. »

Mon travail (et mon banc « courbe de masse ») raisonnait comme si le réseau =
« envoyer un message → réception → ACK → travail fait ». **C'EST FAUX ET RÉDUCTEUR.**
La livraison de message est **UNE fonction**, pas le but primaire.

Le réseau ToM est un **organisme vivant à rôles tournants** :
- **L1** — ancre l'état, **n'arbitre JAMAIS** (décision LOCKED #3).
- **Sous-réseaux éphémères** (subnets) qui se forment/dissolvent.
- **Relais** — forwardent (pass-through stateless), ne stockent pas.
- **Backup** — détiennent les messages des absents (métaphore virus, TTL 24h,
  auto-réplication/auto-suppression, ADR-009).
- **Observateurs** — surveillent.
- **Validation** / **validateurs** — valident (pas un business, une contribution).
- **Carnet de rendez-vous** — les détenteurs de slots DHT qui mettent en lien deux
  inconnus (comme un DNS, MAIS décentralisé ET **le rôle TOURNE**).
- **Rôles réseau-imposés** selon la CONTRIBUTION, réputation à fade, « l'arroseur
  arrosé » (LOCKED #4/#5). Tous les nœuds = même code (ADR-006), le rôle vient de la
  topologie + contribution, pas d'un choix.

**MANDAT** : ces rôles doivent **transpirer** dans mon analyse ET être **incorporés aux
scénarios de test**. Un banc qui ne teste que send/ACK entre nœuds homogènes est aveugle à
90% du réseau. Il FAUT tester : un message qui **transite un relais** (multi-hop), un
message **backupé** pour un absent puis livré au retour, l'**assignation/rotation de rôles**
sous contribution, la **découverte via rendez-vous** (deux inconnus mis en lien), la
**formation de subnets**, l'**ancrage L1**, les **observateurs/validateurs**.

## 1. PREMIÈRE ACTION OBLIGATOIRE (avant de coder quoi que ce soit)

Relire **l'intégralité des notes**, à travers le prisme des rôles, **avec l'outil QMD**
(`mcp__qmd__search` / `vector_search` / `deep_search` / `get`) — c'est le moteur de
recherche local sur les markdown : il renvoie des extraits ciblés au lieu de fichiers
entiers → **on lit tout SANS cramer les tokens**. `Read` sur un .md = seulement quand la
ligne exacte est connue (offset+limit).
- ⚠️ Vérifier que le qmd servi = index `tom` (le vault DU REPO). Si les collections
  affichées sont celles d'1RR/partie politique → STOP, voir mémoire `qmd-index-scope-tom`.
  Si `docs/` du repo n'est pas indexé : `qmd collection add docs --name docs --mask
  "**/*.md" --index tom && qmd embed --index tom` puis chercher.
- Docs socles : `docs/tom-whitepaper-v1.md` (§5 économie usage/contribution, §6.5 arroseur,
  les rôles), `_bmad-output/planning-artifacts/design-decisions.md` (7 LOCKED),
  `docs/MISSION.md`, `docs/plans/charte-cibles-agressives.md`, la mémoire `tom-roles-model`
  et `tom-vision-cible`.
- Objectif : que les rôles + la L1 + les subnets + le rendez-vous tournant soient
  **présents dans ma tête ET mes scénarios**, pas juste le chemin message→ACK.

## 2. ÉTAT DE LA FLOTTE (au moment du handoff)

- **5 nœuds UNIFORMES en build 137** : Mac, iPad, iPhone Malik, iPhone Laura (build+deploy
  fait cette session), NAS (tom-chat redéployé). Le re-probe 15s (Lot C build 135) est
  DONC enfin en terrain → task #6 (re-mesurer I9b) devient possible.
- **Flotte PURGÉE** : `/reset?level=network` sur les 4 apps + restart NAS. Le reset TIENT
  maintenant (Mac retombé 355→3 fantômes et stable).
- ⚠️ **Apple TV (« Séjour ») retirée**, remplacée par iPhone Laura (.49).

## 3. INCIDENT QUE J'AI CAUSÉ (leçon gravée)

Mon banc `scenario_courbe.rs` utilisait `RuntimeConfig::default()` → `enable_dht: true`
(`runtime/mod.rs:130`). Mes dizaines de nœuds in-process éphémères ont **publié au
rendez-vous DHT PARTAGÉ** → ~150 fantômes loopback injoignables sur la vraie flotte.
**CORRIGÉ** : le banc fait maintenant `n0_discovery(false).local_discovery(false)` +
`enable_dht: false` (le trio de `--isolated`). **RÈGLE** : tout nœud de test coupe le
rendez-vous partagé, sinon il pollue le terrain (cf runbook + mémoire
`topology-poisoning-ghost-peers-2026-07-17`).

## 4. TRAVAIL EN COURS (2 commits LOCAUX non poussés : 17c6204 + d27bd7c)

Banc « courbe de masse » Phase 1 in-process (`crates/tom-stress/src/scenario_courbe.rs`,
commande `tom-stress courbe`). Révisé après revue oracle 4 agents. **RESTE À FAIRE** :
- **Recadrer** : ce banc ne juge que l'**intégrité de livraison** (perte/doublon/
  herméticité) — PAS le débit à saturation ni la latence (runtime tokio partagé). Et
  surtout : **il ignore les rôles** (§0). Le prochain banc doit exercer relais/backup/
  rendez-vous/rotation, pas juste send/ACK.
- Mettre à jour `docs/plans/banc-courbe-masse.md` §2bis (chiffres révisés + drain
  quiescence + attribution `tom-quinn` pas `tom-quinn-proto` pour l'`unreachable`).
- Repasser `/review-oracle` (le diff a beaucoup changé) + gate `clippy+test workspace` +
  push. **NE PAS pousser sans ça.**
- Findings terrain ouverts : au N=20 in-process, effondrement = **saturation de MON seul
  Mac** (runtime partagé), pas le protocole → d'où l'intérêt du multi-host RÉEL.

## 5. UX À TRAITER (notes Malik, apps Swift `TomNode/Views/SettingsView.swift`)

1. **Boutons de reset dans les Réglages = inertes** : pas de retour haptique NI visuel →
   on dirait qu'ils ne font rien. Ajouter feedback haptique + état visuel (spinner/toast
   « réseau purgé »). Malik : « faut voir que ça fait quelque chose ».
2. **App auto-démarre 2 s après lancement** (pas besoin du bouton Démarrer) — noté ;
   confirmer si c'est voulu ou à rendre explicite.

## 6. CHANTIERS DE FOND (rappel, priorisés par la charte)

- Le VRAI banc = exercer les rôles sous charge (§0), mesuré au collecteur (fiable), sur
  la flotte RÉELLE multi-host (pas in-process mono-runtime).
- Task #3 (I10 hors LAN), task #6 (I9b re-probe 15s maintenant en terrain).
- Charte `docs/plans/charte-cibles-agressives.md` = le JUGE ; gaps honnêtes actés.

> Ordre : (1) relire les notes via QMD sous le prisme rôles → (2) finir doc+review+push du
> banc → (3) concevoir le banc « rôles sous charge » → (4) UX Réglages. Vault-first, §5
> anti-hallucination, loop-master obligatoire.
