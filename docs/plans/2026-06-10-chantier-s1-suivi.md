# Chantier S1 — API SDK Rust (façade tom-sdk) · Suivi d'exécution

> Démarré : 2026-06-10 22:18 · Référence : `2026-06-10-roadmap-sdk.md` (Phase S1) · Précédent : chantier S0 clôturé (`2026-06-10-chantier-s0-suivi.md`)
> Règle : un commit atomique par tâche · gate clippy+test workspace avant clôture · pas de push (dette handoff §25)
> Décisions appliquées : D1 (tag git), D5 (façade `tom-sdk` fine au-dessus de tom-protocol)

## Tableau de bord

| Tâche | Description | Statut | Commit |
|---|---|---|---|
| S1.0 | Crate `tom-sdk` : squelette + TomClient haut niveau | ⏳ | — |
| S1.1 | Étanchéité : aucun type forké dans l'API tom-sdk (wrappers PeerAddr/RelayAddr/PathChange) | ⏸ | — |
| S1.2 | Erreurs : TomSdkError documentée + #[non_exhaustive] | ⏸ | — |
| S1.3 | Docs : deny(missing_docs), examples/, README crate, métadonnées Cargo | ⏸ | — |
| S1.4 | Corrections tom-protocol : unwrap() metrics, allow(dead_code) | ⏸ | — |
| S1.V | Validation : projet externe fictif compile les exemples + gate workspace | ⏸ | — |

## Journal de chantier

### 2026-06-10 22:18 — Ouverture

Principe directeur (D5) : `tom-protocol` reste le moteur, `tom-sdk` est le contrat public.
La façade ne re-exporte NI `RuntimeCommand`, NI les types des crates forkés (tom-connect/tom-transport).
Conception de l'API basée sur l'usage réel de tom-tui (consommateur de référence identifié à l'audit).

<!-- Entrées ajoutées au fil de l'exécution -->
