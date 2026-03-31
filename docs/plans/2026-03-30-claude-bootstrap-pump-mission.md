# Mission Claude — Pompe d’amorçage ToM

## Mode de travail
**Lecture / analyse / design uniquement pour l’instant.**

Ne modifie pas le code du repo.
Ne propose pas de patch direct.
Travaille comme **contre-architecte** pour challenger le design.

## Objectif
Évaluer et challenger la future **pompe d’amorçage** de ToM Protocol.

Le but n’est pas de réinventer tout le protocole, mais de répondre à cette question :

> Comment rendre le démarrage du réseau **distribué, non figé, non manuel**, sans retomber dans un bootstrap hardcodé ou centralisé ?

## Contexte déjà établi
- le projet vise un réseau réellement autonome
- le test **strict organic** ne converge pas encore
- un test **seed handoff runtime-only** a fonctionné entre Mac et NAS
- l’Apple TV ne peut pas dépendre d’une saisie manuelle de longs NodeId
- conclusion actuelle :
  - il n’existe pas de vrai réseau sans aucune fonction d’amorçage
  - il faut une **pompe d’amorçage distribuée, tournante, non hardcodée, invisible pour l’utilisateur**

## Ce que Copilot fait en parallèle
Copilot travaille déjà sur la **cartographie repo-réaliste** et les **points de branchement concrets** dans les crates existantes.

Donc ta mission n’est PAS de refaire cette cartographie en détail.
Ta mission est de **casser les angles morts** :
- architecture
- résilience
- sécurité
- UX invisible
- convergence
- dégradation / rollback
- partitions LAN / WAN

## Fichiers à lire en premier
Lis ces fichiers avant de répondre :

### Vision / cadrage
- `docs/plans/2026-03-30-parallel-copilot-claude-worksplit.md`
- `docs/plans/2026-03-30-bootstrap-pump-module.md`
- `docs/plans/2026-03-30-organic-seed-handoff-test.md`
- `docs/plans/2026-03-21-macro-roadmap-realignment.md`
- `docs/plans/2026-03-30-rattrapage-bmad-claude-chef-de-projet.md`

### Points techniques déjà identifiés par Copilot
- `crates/tom-transport/src/config.rs`
- `crates/tom-transport/src/node.rs`
- `crates/tom-protocol/src/runtime/loop.rs`
- `crates/tom-protocol/src/runtime/mod.rs`
- `crates/tom-protocol/src/runtime/state.rs`
- `crates/tom-connect/src/address_lookup/mdns.rs`
- `crates/tom-connect/src/address_lookup/pkarr/dht.rs`
- `crates/tom-connect/src/socket/transports/relay/actor.rs`
- `crates/tom-integration-tests/tests/peer_present_auto_discovery.rs`

## Ta mission exacte
Je veux une note courte mais dense, avec :

1. **Variante A / B / C** de pompe d’amorçage réaliste
2. pour chaque variante :
   - principe
   - avantages
   - inconvénients
   - risques
   - conditions d’échec
3. **ta recommandation nette**
4. les **invariants à ne pas casser**
5. une **check-list d’implémentation incrémentale**

## Contraintes non négociables
- pas de bootstrap fixe hardcodé produit
- pas de point central permanent
- le protocole doit rester **invisible pour l’utilisateur final**
- le système doit pouvoir fonctionner avec des rôles tournants
- la solution doit être **actionnable dans le code actuel**, pas purement théorique
- privilégier une montée en puissance :
  - LAN d’abord si pertinent
  - puis propagation
  - puis survie au retrait du seed initial

## Questions auxquelles tu dois répondre
- Est-ce que **LAN-first (mDNS)** doit être la phase 1 obligatoire ?
- Quel doit être l’ordre réel entre :
  - mDNS
  - PeerPresent relay-assisted
  - DHT/Pkarr
  - relay discovery
- Faut-il une vraie **machine de phase d’amorçage** ou juste un assemblage opportuniste de signaux ?
- Comment éviter qu’une pompe d’amorçage devienne un nouveau point d’attaque ?
- Comment gérer :
  - réseau local vide
  - seed initial disparu
  - partition réseau
  - faux signaux / spam / pollution
  - appareil contraint type Apple TV
- Qu’est-ce qui doit rester strictement **interne et invisible** pour l’utilisateur ?

## Format attendu
Réponse structurée ainsi :

### Variante A
...

### Variante B
...

### Variante C
...

### Recommandation
...

### Invariants
...

### Plan incrémental
...

## Important
Ne pars pas dans une dissertation générale sur le P2P.
Je veux un retour **tranché, critique, concret, exploitable** pour guider l’implémentation immédiate.
