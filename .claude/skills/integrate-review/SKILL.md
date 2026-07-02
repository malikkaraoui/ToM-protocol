---
name: integrate-review
description: "Ferme la boucle d'un handoff inter-LLM. Si response.content est null, va chercher les inline Copilot sur la PR GitHub associée et les retranscrit automatiquement. Puis classe les points, applique les fixes, génère l'intégration. Utiliser quand Copilot a reviewé une PR portant un handoff."
figure: Greffier
---

# Integrate Review

> Le Greffier revient avec le dossier de l'autre atelier sous le bras.
> Il ouvre, trie, classe : retenu, à garder en tête, écarté.

Fermeture de boucle inter-LLM. Copilot/GPT a reviewé une PR →
récupérer les inline GitHub, vérifier, retranscrire, appliquer, committer.

## Procédure

### Étape 0 — Trouver le handoff et les inline Copilot

```bash
# Handoff le plus récent non intégré
ls -lt docs/handoffs/*.json | grep -v _template | head -5
```

Identifie le handoff cible (argument `/integrate-review <fichier>` ou le plus récent
avec `response.content: null`).

**Si `response.content` est non-null** → aller directement à l'Étape 2.

**Si `response.content` est null** → aller chercher les inline Copilot via GitHub API :

```bash
# Lister les PRs ouvertes/récentes pour trouver celle liée au handoff
gh pr list --state all --limit 10 --json number,title,headRefName

# Récupérer les inline reviews Copilot de la PR cible
gh api repos/<owner>/<repo>/pulls/<PR_NUM>/comments \
  --jq '.[] | select(.user.login | test("copilot"; "i")) | {id, path, line, body}'

# Récupérer aussi la review globale Copilot
gh api repos/<owner>/<repo>/pulls/<PR_NUM>/reviews \
  --jq '.[] | select(.user.login | test("copilot"; "i")) | {id, body, state}'
```

**Vérification anti-hallucination obligatoire** — pour chaque commentaire inline,
vérifier que le point soulevé correspond à un vrai problème :

| Type de commentaire | Vérification |
|---|---|
| Syntaxe/formatage (ex: `||` dans Markdown) | `grep '<pattern>' <fichier>` → 0 match = hallucination |
| Bug logique (ex: variable incorrecte) | Lire le code autour de la ligne citée |
| Missing field / null value | Vérifier dans le fichier JSON |
| Typo/cohérence linguistique | Chercher le terme dans le fichier |

**Tout commentaire non vérifiable est écarté** (ne jamais accepter sans grep/lecture).

Après vérification, écrire `response.content` dans le handoff JSON :

```json
"response": {
  "model": "github-copilot-pr-reviewer",
  "content": "<synthèse des points valides + verdict global>"
}
```

Règles pour `response.content` :
- Inclure le **verdict** sur la question posée dans `from.question`
- Lister les **points valides** retenus avec mécanisme de défaillance précis
- Lister les **hallucinations détectées** (pour traçabilité)
- Si Copilot n'a pas répondu au fond (que des commentaires procéduraux/forme) : l'écrire honnêtement + recommander second reviewer

### Étape 1 — Identifier les fixes à appliquer

Pour chaque point valide dans `response.content`, classer :

- **À implémenter maintenant** : bug réel, actionnable
- **À garder en tête** : valide mais pas prioritaire
- **Écarté** : hallucination, déjà traité, contredit une décision

### Étape 2 — Appliquer les fixes

Pour chaque point **à implémenter maintenant** :
- Identifier le fichier et la ligne
- Faire le fix minimal (Edit ciblé)
- `npm test` doit passer
- Commit atomique sur la branche du PR

### Étape 3 — Écrire le champ `integration`

Dans le fichier handoff `.json`, remplace `"integration": null` par :

```json
"integration": {
  "status": "completed",
  "validatedTarget": "npm test + bash scripts/pre-push-gate.sh",
  "summary": "...",
  "appliedFixes": [
    "<description fix 1> (commit <sha>)"
  ],
  "rejectedSuggestions": [
    "<commentaire hallucination> — rejeté : <grep ou preuve>"
  ],
  "notes": [
    "Tests : N passed · 0 failed",
    "Gate pre-push : verte"
  ]
}
```

Si aucun fix applicable (review de fond absente) :
```json
"integration": {
  "status": "no-review-content",
  "summary": "...",
  "appliedFixes": ["Aucun"],
  "notes": ["Prochaine étape : second avis via /la-bise"]
}
```

### Étape 4 — Valider et committer

```bash
node test/validate-handoff.js docs/handoffs/<fichier>.json
git add docs/handoffs/<fichier>.json [<autres fichiers fixes>]
git commit -m "docs: intégrer review Copilot handoff <sujet>"
git push origin <branche>
```

### Étape 5 — Répondre aux commentaires GitHub

Pour chaque inline Copilot sur la PR :

```bash
# Commentaire accepté
gh api repos/<owner>/<repo>/pulls/<PR_NUM>/comments/<ID>/replies \
  -X POST -f body="✅ Appliqué — commit \`<sha>\`. <explication>"

# Commentaire rejeté (hallucination)
gh api repos/<owner>/<repo>/pulls/<PR_NUM>/comments/<ID>/replies \
  -X POST -f body="❌ Hallucination — vérifié \`grep '<pattern>' <fichier>\` = 0 match. <explication>"
```

### Étape 6 — Merger la PR (si gate verte)

```bash
bash scripts/pre-push-gate.sh
gh pr merge <PR_NUM> --merge --delete-branch
```

## Règles

- Ne jamais accepter un commentaire Copilot sans vérification (§5 prime)
- Hallucinations connues fréquentes : `||` dans Markdown, imports fictifs, APIs inexistantes
- La source réelle de `response.content` est toujours `github-copilot-pr-reviewer` (inline GitHub)
- Le SWE agent (`copilot-swe-agent[bot]`) remplit parfois le handoff directement — dans ce cas, aller directement à l'Étape 2
- `integration` est rempli par Claude, jamais par Copilot
- `meta` et `from` sont intouchables (ancrés à la création du handoff)
