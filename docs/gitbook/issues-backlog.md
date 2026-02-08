# Backlog d’issues

## Objectif

Maintenir en permanence des tâches prêtes à être transformées en issues GitHub.

## Où les trouver

- Backlog permanent (source) : https://github.com/malikkaraoui/ToM-protocol/blob/main/.github/ISSUE_BACKLOG.md
- Issues GitHub : https://github.com/malikkaraoui/ToM-protocol/issues

## Comment s’en servir (simple et efficace)

1. Choisis une issue **micro** ou **small** (30–60 min) pour garder le rythme.
2. Respecte la direction des dépendances : **demo → sdk → core**.
3. Si tu touches au comportement, ajoute au moins un test.
4. Une PR = un changement logique (petite surface, facile à reviewer).

{% hint style="info" %}
Le backlog n’est pas un « reste à faire ». C’est un **moteur de contribution** : il doit rester alimenté, trié et actionnable.
{% endhint %}

## Exemples (types de tâches)

- Refactor discovery : simplifier l’API EphemeralSubnetManager
- Refactor routing : extraire la validation des messages hors du Router
- CI : coverage, audit dépendances, build size tracking
- Vérification : audit du signaling server, audit de cohérence `TomError`

## Principe

On préfère des tickets concrets, testables et bornés, plutôt que des « grandes intentions ».
