# Prochaine session — le carnet de rendez-vous (2 chantiers de conception)

> Ouverts par Malik le 2026-07-21, à la suite de l'autopsie OOM Freebox.
> Design-first (feature protocolaire LOCKED → doc de conception AVANT de coder,
> cf `prisme-des-roles.md` ligne 27/88 : « la rotation n'existe pas, aucun design »).

## Contexte : d'où ça vient
L'OOM répété de la Freebox a révélé que le rendez-vous actuel (ADR-010) est un
**DHT Mainline mondial diffusif** : 8 slots partagés, chaque nœud publie sa carte
et lit tous les slots → chacun voit défiler tout le monde. Le nœud stable 24/7
(Freebox) ramasse tout (789 entrées, 787 inconnus) et le paie en mémoire. Ce
n'est pas le « rôle tournant à quelques détenteurs » de la vision.

## Chantier 1 — Le rôle de carnet doit TOURNER
**Verbatim Malik** : « le carnet avec les différents nœuds […] doit tourner
demain et ne doit jamais être chez la même personne […] on pointe toujours du
doigt la Freebox ».

- **État** : le rôle « détenteur du carnet de rendez-vous » n'existe PAS comme
  rôle ToM. Il est sous-traité au DHT Mainline mondial (aucun détenteur ToM
  désigné, aucune rotation). Conforme à la vision « comme le DNS **mais ça
  tourne** » : NON implémenté.
- **À concevoir** : qui détient un slot/carnet, comment le rôle est assigné par
  la topologie + contribution (comme les autres rôles réseau-imposés), comment
  il TOURNE (rotation imprévisible, cf murs #1/#2 Fable — cascade + entropie),
  comment on reste à « quelques dizaines » de détenteurs et pas mondial.

## Chantier 2 — Le carnet ne doit ramasser que le NÉCESSAIRE
**Verbatim Malik** : « je vois pas pourquoi je garderais autant de fantômes dans
mon carnet de contacts. Un contact qui a déjà un ou deux points de rendez-vous
ne doit pas se retrouver dans les carnets de Monsieur-Madame-tout-le-monde. »

- **Modèle-vision** : je publie mon adresse chez **mes quelques hôtes de
  rendez-vous** ; seul quelqu'un qui me **cherche** me trouve via eux → carnet
  ciblé, petit. Un nœud « déjà placé » (qui a ses hôtes) ne pollue pas les autres.
- **Modèle-actuel** : slot partagé lu par TOUS → **diffusion de facto** à tous
  les lecteurs, qu'ils me cherchent ou non. La fuite mémoire est le prix de cette
  diffusion, payé par le nœud stable.
- **À concevoir** : découverte par RECHERCHE ciblée (je cherche X → je demande à
  ses hôtes) plutôt que par balayage-de-tout ; critère « ai-je besoin de ce
  pair ? » avant de le garder ; TTL/purge alignés sur le besoin réel.

## Lien avec le bug mémoire (à ne pas confondre)
Le bug transport/relais (autopsie du 21/07, `tom-freebox-oom-carnet-rendezvous`)
est une couche SÉPARÉE : il faut le corriger pour éteindre l'OOM (mitigation :
NAS `--isolated`), mais il ne résout PAS ces deux chantiers de conception. Les
deux sont nécessaires : sans (1)+(2), la Freebox restera l'aspirateur de la flotte.
