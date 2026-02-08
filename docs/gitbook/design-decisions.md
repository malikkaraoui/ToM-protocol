# Décisions verrouillées (les « 7 locks »)

## Statut

Ces décisions sont considérées comme finales et non négociables. Elles définissent le caractère de ToM.

{% hint style="warning" %}
Ces règles sont des **invariants**. Toute proposition qui les contredit est hors‑scope (même si elle est « cool »).
{% endhint %}

## Les 7 décisions

| # | Règle | Ce que ça implique |
|---:|---|---|
| 1 | **Delivery = ACK** | Un message est « délivré » si et seulement si le destinataire final émet un ACK. Les UI/metrics s’alignent sur cette définition. |
| 2 | **TTL = 24h puis purge** | Effort maximal **borné**. Pas d’historique infini, pas d’exception. |
| 3 | **L1 observe et ancre** | La L1 n’arbitre pas le réseau. Elle sert d’observateur/horodatage, pas de juge. |
| 4 | **Réputation progressive** | Le passé existe mais s’efface. Pas de condamnation permanente. |
| 5 | **Anti‑spam progressif** | Réaction continue et graduelle. Pas de seuil magique, pas d’exclusion définitive. |
| 6 | **Invisibilité** | ToM est une couche protocolaire : l’utilisateur final ne « voit » pas ToM. |
| 7 | **Scope fondation** | ToM est une fondation universelle (type TCP/IP), pas une application. |

## Source

- Document complet : https://github.com/malikkaraoui/ToM-protocol/blob/main/_bmad-output/planning-artifacts/design-decisions.md
