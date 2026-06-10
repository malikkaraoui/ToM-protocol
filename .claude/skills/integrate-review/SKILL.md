---
name: integrate-review
description: "Ferme la boucle d'un handoff inter-LLM. Lit la réponse de Copilot/GPT dans docs/handoffs/, classe les points, génère la section Intégration et une checklist d'actions. Utiliser quand Copilot a répondu à un handoff."
figure: Greffier
---

# Integrate Review

> Le Greffier revient avec le dossier de l'autre atelier sous le bras.
> Il ouvre, trie, classe : retenu, à garder en tête, écarté.

Fermeture de boucle inter-LLM. Copilot/GPT a répondu → lire, trier,
transformer en actions concrètes.

## Procédure

### Étape 1 — Trouver le handoff

Lis le dernier fichier modifié dans `docs/handoffs/` (excluant
`_template.md` et `README.md`) :

```bash
ls -lt docs/handoffs/*.md | grep -v _template | grep -v README | head -1
```

Si la section « Réponse de » est vide → « Copilot n'a pas encore
répondu. Attends sa réponse puis relance /integrate-review. »

### Étape 2 — Lire et analyser la réponse

Lis la réponse complète de Copilot/GPT. Pour chaque point soulevé,
classe-le dans une des 3 catégories :

- **Retenu — à implémenter** : le point est valide ET actionnable
  maintenant. Inclure l'action concrète.
- **Retenu — à garder en tête** : le point est valide MAIS pas
  actionnable immédiatement (manque de contexte, pas prioritaire,
  dépend d'un besoin futur). Expliquer pourquoi on ne le fait pas
  maintenant.
- **Écarté** : le point n'est pas pertinent, est déjà traité, ou
  contredit une décision prise. Expliquer pourquoi.

### Étape 3 — Écrire la section Intégration

Dans le fichier handoff, remplace le contenu de la section
« Intégration » par :

```markdown
## Intégration (Claude, YYYY-MM-DD)

### Retenu — à implémenter

| # | Point | Action |
| --- | --- | --- |
| ... | ... | ... |

### Retenu — à garder en tête

| Point | Pourquoi pas maintenant |
| --- | --- |
| ... | ... |

### Écarté

| Point | Pourquoi |
| --- | --- |
| ... | ... |

### Verdict

[1-2 phrases sur la qualité globale de la review + le point le plus
actionnable]
```

### Étape 4 — Générer la checklist d'actions

Pour chaque point « Retenu — à implémenter », créer une tâche dans
TodoWrite pour tracker l'exécution.

### Étape 5 — Proposer l'exécution

"Intégration terminée. [N] points retenus à implémenter :

1. [action 1]
2. [action 2]
3. ...

Tu veux que j'implémente maintenant, ou on priorise d'abord ?"

### Étape 6 — Committer l'intégration

```bash
git add docs/handoffs/[fichier].md
git commit -m "docs: integrer review [sujet] de Copilot/GPT"
```

## Règles

- Ne jamais rejeter un point sans explication
- Ne jamais accepter un point aveuglément (vérifier qu'il est pertinent
  et cohérent avec les décisions prises)
- La section Intégration est remplie par Claude, pas par l'utilisateur
- Chaque point retenu doit avoir une action concrète, pas un vague
  "à améliorer"
- §5 prime : ne pas inventer des actions pour remplir le tableau
