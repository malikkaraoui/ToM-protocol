# PROMPT DE REPRISE — le rôle « carnet de rendez-vous TOURNANT » (pas de point fixe)

> Rédigé 2026-07-21 soir. Chantier de CONCEPTION (design-first, protocolaire
> LOCKED). But : que le rendez-vous devienne un vrai rôle ToM tournant à quelques
> détenteurs, ciblé — **jamais un point fixe**. Détail des 2 chantiers :
> `docs/plans/PROCHAINE-SESSION-carnet-rendezvous.md` (à lire avec ce prompt).

## 0. PREMIÈRE ACTION
Lire, avec QMD (`mcp__qmd__status` témoin d'abord) :
- `docs/plans/PROCHAINE-SESSION-carnet-rendezvous.md` (les 2 chantiers, verbatim Malik)
- `docs/plans/prisme-des-roles.md` ligne 27/88 (écart « rotation n'existe pas »)
- `docs/tom-whitepaper-v1.md` (les rôles, §3.4 rotation pseudo-aléatoire, §4.3)
- mémoire `tom-freebox-oom-carnet-rendezvous` (pourquoi ça compte : le point fixe
  = l'aspirateur qui OOM) et `reseau-organisme-roles-pas-tuyau`.
- ADR-010 (`CLAUDE.md`) : l'état actuel = 8 slots DHT mondiaux partagés.

## 1. LE MANDAT (verbatim Malik, à garder à l'esprit)
> « le carnet avec les différents nœuds […] doit tourner demain et ne doit jamais
> être chez la même personne […] on pointe toujours du doigt la Freebox. »

> « un contact qui a déjà ses points de rendez-vous ne doit pas se retrouver dans
> le carnet de Monsieur-Madame-tout-le-monde. Le modèle-vision : je publie mon
> adresse chez mes quelques hôtes, et seul quelqu'un qui me cherche me trouve via
> eux → carnet ciblé, petit. »

**Le but : PAS DE POINT FIXE.** Comme le DNS, mais ça TOURNE.

## 2. LES DEUX CHANTIERS
### Chantier 1 — Le rôle carnet doit TOURNER
Aujourd'hui : DHT Mainline mondial figé, aucun détenteur ToM désigné, aucune
rotation. À concevoir : qui détient un slot/carnet, assignation par topologie +
contribution (comme les autres rôles réseau-imposés, ADR-006), rotation
imprévisible (cascade + entropie — murs #1/#2 red-team Fable, cf charte cibles
agressives), maintien à « quelques dizaines » de détenteurs, pas mondial.

### Chantier 2 — Le carnet ne ramasse que le NÉCESSAIRE
Aujourd'hui : slot partagé lu par tous → diffusion de facto. À concevoir :
découverte par RECHERCHE ciblée (je cherche X → j'interroge SES hôtes) plutôt
que balayage-de-tout ; un nœud « déjà placé » ne pollue pas les carnets des autres.

## 3. MÉTHODE (design-first, non négociable)
- **Doc de conception AVANT tout code** (règle projet : feature protocolaire
  LOCKED/red-teamée → design-doc d'abord ; mémoire `design-doc-before-coding-protocol-features`).
- Red-team le design AVANT de coder : fragmentation du réseau si mal fait,
  squatting de slot, un détenteur malveillant, partition, rotation qui perd des
  liens. Croiser aux écrits fondateurs (whitepaper/MISSION/design-decisions) —
  ne pas réinventer ce qui est déjà LOCKED (7 décisions : L1 n'arbitre pas,
  réputation fade, sprinkler, invisibilité, etc.).
- Multi-agent utile : panel d'approches indépendantes (DHT-avec-rotation vs
  rendez-vous-par-recherche vs hybride), juge, synthèse.
- Lien avec le bug OOM : le fix mémoire (`PROMPT-fix-freebox-oom.md`) et ce
  chantier sont SÉPARÉS mais complémentaires — borner le carnet de contacts
  (fix) atténue le symptôme ; le rendez-vous tournant+ciblé (ce chantier)
  supprime la cause-conception (le point fixe qui aspire). Les DEUX sont
  nécessaires.

## 4. LIVRABLE DE LA PREMIÈRE SESSION
Un doc de conception (`docs/plans/design-carnet-rendezvous-tournant.md`) qui :
- pose le modèle cible (détenteurs tournants + découverte par recherche),
- red-teame les modes de défaillance,
- trace la migration depuis l'ADR-010 actuel (8 slots mondiaux),
- définit les scénarios de test (le banc R4 devient la non-régression : deux
  inconnus se trouvent, MAIS via le nouveau mécanisme tournant).
Pas de code protocolaire avant validation de ce doc par Malik.
