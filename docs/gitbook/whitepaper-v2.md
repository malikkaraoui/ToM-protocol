# Livre blanc v2 (plan)

## Intention

Le whitepaper v2 ne doit pas être une promesse marketing.
Il doit être un document de cadrage : invariants, preuves, zones ouvertes, et roadmap de transition (bootstrap → autonomie).

## Structure proposée

1) Ce qui est vrai maintenant (preuves)
- Ce que la démo et le code valident déjà : relay, multi-relay, ACK, rerouting, E2E, discovery, subnets, failover
- Comment c’est vérifié (tests unitaires + E2E)

2) Invariants non négociables
- Les 7 décisions verrouillées (delivery=ACK, TTL, L1 anchor, réputation progressive, anti-spam progressif, invisibilité, scope)

3) Ce qui reste volontairement ouvert
- Paramètres exacts de contribution/usage
- Anti-sybil à grande échelle
- Détails DHT / suppression progressive du signaling

4) Roadmap de suppression du bootstrap
- Étapes concrètes et critères de validation

## Sources à intégrer

- Décisions verrouillées : https://github.com/malikkaraoui/ToM-protocol/blob/main/_bmad-output/planning-artifacts/design-decisions.md
- Architecture (ADR-002) : https://github.com/malikkaraoui/ToM-protocol/blob/main/_bmad-output/planning-artifacts/architecture.md
- Rétro consolidation : https://github.com/malikkaraoui/ToM-protocol/blob/main/_bmad-output/implementation-artifacts/epic-4-8-retro-2026-02-07.md
