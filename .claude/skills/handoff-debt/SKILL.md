---
name: handoff-debt
description: "Affiche la dette §25 calculée depuis git (commits + lignes + jours depuis dernier handoff intégré), liste les commits non reviewés, et génère un draft de handoff pré-rempli. À utiliser quand le bandeau [HANDOFF DEBT §25] apparaît."
figure: Mohamed
---

# Handoff Debt

> Mohamed 📋 ouvre son registre, compte ce qui doit être reviewé,
> et prépare le dossier vide. Claude n'a plus qu'à poser la question.
>
> *"La dette non mesurée est de la dette non payée."*

Affiche l'état §25 et génère un handoff draft prêt à compléter.

## Procédure

### Étape 1 — Afficher la dette (depuis git, jamais JSON)

```bash
bash scripts/handoff-debt.sh
```

Output attendu : commits depuis dernier handoff intégré, lignes ajoutées/supprimées, jours écoulés, seuils, dépassement.

### Étape 2 — Si dette dépassée, lister les commits concernés

```bash
# Récupère le range depuis handoff-debt
RANGE=$(bash scripts/handoff-debt.sh --json | python3 -c "import sys,json;print(json.load(sys.stdin)['reviewedRange'])")
git log --oneline "$RANGE"
git diff --stat "$RANGE"
```

### Étape 3 — Générer le draft

Lancer `bash scripts/handoff-draft.sh <slug>` qui crée un fichier pré-rempli dans `docs/handoffs/YYYY-MM-DD-<slug>.md` avec :

- Frontmatter Date/Type/Priorité
- Section "Contexte" pré-remplie avec la liste des commits du range
- Section "Fichiers à lire" pré-remplie avec les fichiers modifiés
- Section "Question précise" laissée vide (à remplir par Claude)
- Section "Réponse de : Copilot/GPT" avec placeholder
- Section "Intégration" vide

### Étape 4 — Compléter et livrer

Claude complète la section "Question précise" avec UNE question opinionée, puis :

- annonce à l'utilisateur : `Handoff prêt : docs/handoffs/YYYY-MM-DD-<slug>.md — à donner à Copilot`
- le user passe le fichier à Copilot, reçoit la réponse, Claude l'intègre via `/integrate-review`

## Règles

- **Source de vérité = git**, jamais un JSON ou un compteur `/tmp`. Si les résultats semblent aberrants, vérifier que `docs/handoffs/` n'a pas été altéré et que la section `## Intégration` du dernier handoff a bien > 100 caractères de texte réel.
- Le reset de la dette se fait **uniquement** par `/integrate-review` après réponse Copilot intégrée.
- Ne pas générer un handoff vide pour "blanchir" la dette — `test/validate-handoff.js` le rejettera au prochain push.
