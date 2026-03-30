# Module `pompe d'amorçage` — cadrage clair

**Date :** 2026-03-30
**Statut :** cadrage

## Pourquoi ce document

Le mot *bootstrap* est aujourd'hui ambigu.

Dans l'industrie, il signifie souvent :

- quelques serveurs fixes connus d'avance,
- hardcodés ou quasi-hardcodés,
- points d'entrée permanents du réseau.

Ce n'est **pas** la cible ToM.

La cible ToM est différente :

- il existe bien une **fonction d'amorçage**,
- mais cette fonction n'est **pas attachée durablement à des serveurs fixes**,
- elle est **portée temporairement par le réseau**,
- et le rôle d'entrée est **rotatif**.

Autrement dit :

> ToM a bien besoin d'une **pompe d'amorçage**, mais pas d'un **bootstrap central fixe**.

---

## Formulation simple

### Ce qu'on accepte

Une **pompe d'amorçage** est un mécanisme qui aide un nouveau nœud à rencontrer le réseau.

Elle peut être :

- locale (LAN / mDNS / voisinage),
- communautaire (seed vivant fourni par le réseau),
- opportuniste (peer appris par propagation),
- temporaire (amorce runtime),
- tournante (rôle repris par d'autres nœuds).

### Ce qu'on refuse comme cible finale

- un petit groupe de serveurs fixes toujours connus d'avance,
- un bootstrap éternel codé dans le client,
- une dépendance durable à l'infrastructure du porteur initial,
- une saisie manuelle pénible côté utilisateur final.

---

## Métaphore de référence

La bonne image n'est pas "annuaire central".

La bonne image est :

## la secrétaire qui joue à la chaise musicale

- quelqu'un répond bien au téléphone,
- mais ce n'est jamais forcément la même personne,
- cette personne ne décide pas du réseau,
- elle fait juste les présentations,
- puis elle passe le relai,
- et le réseau continue même si elle disparaît.

Donc :

- le **rôle existe**,
- la **personne porteuse du rôle change**,
- la **fonction survit à l'instance**.

---

## Cible long terme

### Niveau 1 — naissance

Le réseau a besoin d'une pompe d'amorçage identifiable.

### Niveau 2 — croissance

La pompe n'est plus unique :

- plusieurs porteurs possibles,
- plusieurs chemins d'entrée,
- reprise possible si un porteur tombe.

### Niveau 3 — autonomie

Le réseau choisit et renouvelle lui-même les porteurs de la fonction d'amorçage.

Le nouveau venu ne sait pas à l'avance qui jouera ce rôle.

### Niveau 4 — maturité

La pompe d'amorçage devient une **fonction émergente du réseau** :

- présente,
- distribuée,
- non attachée à une infra fixe,
- transmissible,
- remplaçable.

---

## Ce que le module doit contenir

Le module `pompe d'amorçage` doit rendre explicites les responsabilités suivantes.

### 1. Apparition initiale

Comment un nœud voit pour la première fois qu'un réseau existe.

Exemples possibles :

- LAN zero-conf,
- seed communautaire vivant,
- rendez-vous topic,
- invite / QR / code court.

### 2. Présentation minimale

Le rôle de la pompe n'est pas de gouverner.

Il est seulement de fournir assez d'information pour que le nouveau nœud :

- découvre des peers,
- découvre des relays,
- découvre des routes utiles,
- entre ensuite dans les mécanismes normaux du réseau.

### 3. Handoff

Une fois l'entrée réalisée, la pompe doit s'effacer.

Le nouveau nœud doit ensuite apprendre par :

- propagation,
- gossip,
- relays appris,
- DHT quand applicable,
- topologie vivante.

### 4. Rotation

Le rôle doit pouvoir changer de porteur.

Le module doit donc penser explicitement :

- éligibilité du porteur,
- perte du porteur,
- remplacement,
- continuité de service.

---

## Ce que le module ne doit pas faire

- ne pas devenir un centre de décision,
- ne pas devenir un annuaire permanent figé,
- ne pas devenir une liste sacrée de serveurs,
- ne pas exposer une UX de saisie pénible à l'utilisateur final,
- ne pas confondre "amorçage" et "contrôle du réseau".

---

## Traduction technique immédiate

À court terme, cela veut dire qu'il faut séparer clairement :

### A. l'amorçage actuel de test

- seed live runtime-only,
- bootstrap ponctuel,
- support de validation.

### B. la vraie pompe d'amorçage cible

- zero-conf LAN si disponible,
- présentation initiale sans saisie longue,
- apprentissage automatique des peers/relays,
- handoff après entrée,
- seed down sans effondrement.

---

## Critère de vérité

On ne pourra dire que la vision ToM est tenue que si :

1. un nouveau nœud rejoint sans saisie pénible,
2. il n'entre pas via un point fixe unique,
3. le porteur de l'amorçage peut disparaître,
4. le réseau continue de vivre,
5. un autre porteur reprend la fonction.

---

## Conclusion

Oui, ToM a besoin d'une **pompe d'amorçage**.

Mais non, cette pompe n'est pas censée être :

- fixe,
- centrale,
- éternelle,
- ou codée comme un bootstrap classique.

La cible est bien :

> **une fonction d'amorçage réelle, explicite, temporaire, tournante, et reprise par le réseau lui-même**.
