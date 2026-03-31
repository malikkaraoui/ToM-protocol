# 2026-03-30 — Travail parallèle Copilot / Claude

## Objectif
Accélérer le design et l’implémentation de la **pompe d’amorçage** sans créer de doublons ni de conflits de branche.

## Principe
Deux agents peuvent travailler sur le **même projet**, mais pas sur le **même sous-problème au même moment**.

On coupe le travail en deux angles complémentaires :

- **Copilot**: cartographie du code existant, points d’intégration réels, plan d’implémentation incrémental dans le repo.
- **Claude**: contre-analyse architecture/produit, angles morts, variantes de design, risques, protocole d’échec et de migration.

## Répartition recommandée

### Piste Copilot
Mission principale :
- auditer les briques déjà présentes (`mDNS`, `PeerPresent`, relay discovery, DHT, surfacing runtime)
- proposer un module **implémentable** dans le code actuel
- identifier les crates/fichiers à toucher
- préparer un plan de livraison par étapes testables

Livrables attendus :
- note d’architecture repo-réaliste
- mapping crate -> rôle -> changements
- stratégie de tests

### Piste Claude
Mission principale :
- challenger le concept de pompe d’amorçage
- identifier les angles morts produit / sécurité / UX / résilience
- proposer 2 ou 3 variantes de design
- expliciter ce qui doit rester invisible pour l’utilisateur
- définir les invariants à ne pas casser

Livrables attendus :
- note critique courte
- comparaison des variantes
- risques majeurs
- recommandations tranchées

## Règles pour éviter la collision

1. **Une seule personne/agent modifie le code à la fois** sur une zone donnée.
2. Claude travaille de préférence en **lecture + design + critique**.
3. Copilot garde la main sur :
   - modifications du repo
   - validation locale
   - synthèse finale
4. Si Claude propose du code :
   - le garder d’abord comme proposition textuelle
   - ne l’intégrer qu’après revue croisée
5. Utiliser soit :
   - **2 branches séparées**, ou mieux
   - **2 worktrees séparés**

## Mode opératoire recommandé

### Option simple
- Claude : analyse/doc uniquement
- Copilot : code + tests + intégration

### Option agressive mais propre
- `worktree A` : Copilot sur implémentation
- `worktree B` : Claude sur doc/design/contre-proposition
- fusion humaine ensuite

## Prompt conseillé pour Claude

Tu rejoins le projet ToM Protocol en parallèle de Copilot.

Contexte important :
- le projet vise un réseau réellement autonome, sans bootstrap fixe hardcodé
- le test “strict organic” ne converge pas encore
- un test “seed handoff” runtime-only a marché entre Mac et NAS
- l’Apple TV ne peut pas dépendre d’une saisie manuelle de longs NodeId
- la conclusion actuelle est : il faut une **pompe d’amorçage distribuée, non figée, non manuelle**, pas l’illusion d’un réseau sans aucune fonction d’amorçage

Ta mission :
1. challenger le concept de pompe d’amorçage
2. proposer 2 à 3 architectures réalistes
3. identifier les angles morts (sécurité, UX, abus, convergence, partitions réseau, LAN vs WAN, rollback)
4. dire clairement quelle variante tu recommandes
5. lister les invariants du projet à respecter

Contraintes importantes :
- rester compatible avec la vision ToM : protocole invisible, pas de point fixe central, rôle tournant, pas de bootstrap hardcodé produit
- être concret et actionnable, pas seulement conceptuel
- privilégier une progression incrémentale : LAN d’abord si pertinent, puis propagation, puis survie au retrait du seed initial

Format de réponse attendu :
- variante A / B / C
- avantages / inconvénients
- risques
- recommandation finale
- check-list d’implémentation

## Décision actuelle
Le meilleur usage de Claude ici n’est pas de lui faire éditer le même code en même temps que Copilot.
Le meilleur usage est de l’employer comme **contre-architecte** pour casser les angles morts pendant que Copilot avance dans le repo réel.
