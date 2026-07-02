---
name: review-copilot
description: "Génère un handoff review structuré pour Copilot/GPT. Utiliser après une feature, un bug fix, ou quand > 100 lignes ont changé. Aussi déclenché automatiquement par §25."
figure: Mohamed
---

# Review Copilot

> Mohamed 📋 passe en arrière-salle, relit les commits un par un,
> compte les lignes, formule la question précise — et dépose le dossier
> sur le bureau de Copilot sans un mot de trop.
>
> *"Un code non challengé n'est pas fini. C'est une bombe à retardement."*

Handoff structuré pour Copilot/GPT, créé dans `docs/handoffs/` au format `.json`.
Le JSON est lu par **Copilot via la PR GitHub** — pas de copier-coller, pas de VS Code.
Flux : branche `handoff/` → commit JSON → push → `gh pr create` → Copilot review PR → `/integrate-review`.

## Procédure

### Étape 1 — Collecter le contexte exhaustif

Exécute silencieusement :

```bash
# 1. Trouver le SHA du dernier handoff intégré (source de vérité)
bash scripts/handoff-debt.sh --json | python3 -c "import json,sys; d=json.load(sys.stdin); print(d['lastIntegratedHandoff']['sha'])"

# 2. TOUS les commits feat/fix depuis ce SHA — aucun ne doit manquer
SHA_FROM="<résultat ci-dessus>"
git log "${SHA_FROM}..HEAD" --format="%h %s" | grep -E "^[a-f0-9]+ (feat|fix|refactor)"

# 3. Stats globales du range
git diff --stat "${SHA_FROM}..HEAD" | tail -1

# 4. SHA HEAD courant
git rev-parse HEAD
```

**Règle absolue : le champ `from.context` doit mentionner TOUS les commits feat/fix du range, pas seulement les derniers visibles.**
Infère automatiquement le sujet depuis ces commits.
**Ne pas demander le sujet à l'utilisateur — déduire depuis git log.**

### Étape 2 — Créer la branche handoff

```bash
SLUG="<sujet-slug-depuis-commits>"   # ex: atelier-config-features-agents
BRANCH="handoff/$(date +%Y-%m-%d)-${SLUG}"
git checkout -b "$BRANCH"
```

### Étape 3 — Générer le handoff JSON

**reviewedRange** : `sha-from` = SHA du dernier handoff intégré (résolu via `git rev-parse`). `sha-to` = HEAD courant. Jamais de `..HEAD (uncommitted)`.

Créer `docs/handoffs/YYYY-MM-DD-<sujet-slug>.json` :

```json
{
  "meta": {
    "subject": "[sujet inféré depuis git log]",
    "date": "YYYY-MM-DD",
    "type": "review",
    "priority": "[haute si bug fix critique, moyenne sinon]",
    "reviewedRange": "[sha-from]..[sha-to]"
  },
  "from": {
    "model": "claude-sonnet-4-6",
    "context": "[résumé de ce qui a été fait, basé sur les commits récents]",
    "question": "[UNE question précise sur ce qui est fragile ou manquant]",
    "filesToRead": [
      "[les fichiers les plus modifiés, max 10]"
    ],
    "constraints": [
      "Ne pas proposer de réécrire ce qui fonctionne",
      "Se concentrer sur ce qui manque ou est fragile",
      "INTERDIT de modifier du code source — tu es reviewer, pas développeur",
      "INTERDIT de modifier meta ou from — ces champs sont ancrés par Claude"
    ]
  },
  "response": {
    "model": null,
    "content": null,
    "_instruction": "Écrire la réponse dans le champ 'content'. Ne pas modifier 'meta' ni 'from'. Utiliser l'outil d'édition de fichier. Quand terminé, dire : \"J'ai répondu dans [chemin du fichier].\""
  },
  "integration": null
}
```

### Étape 4 — Committer + pousser la branche + créer la PR

La gate §25 est bypassée sur les branches feature/handoff — pas de deadlock.

```bash
git add docs/handoffs/[fichier].json
git commit -m "docs: handoff review [sujet]"
git push -u origin "$BRANCH"

# Créer la PR — Copilot la reviewera automatiquement
gh pr create \
  --title "handoff: review [sujet]" \
  --body "$(cat <<'EOF'
## Handoff review — [sujet]

Copilot : lis `docs/handoffs/[fichier].json` et réponds dans `response.content`.

**Question :** [valeur de from.question]

**Fichiers clés :** [valeur de from.filesToRead]

**Contraintes :** reviewer uniquement, pas de modification de code ni de meta/from.
EOF
)"
```

### Étape 5 — Sélection du modèle Copilot (optionnel — GPT-5.4 recommandé)

GitHub ne permet pas de forcer un modèle via API/CLI pour les reviews automatiques.
Pour cibler **GPT-5.4** (ou autre modèle spécifique) :

Afficher à l'utilisateur :

```
🎯 Pour forcer GPT-5.4 sur cette review :

  1. Ouvre la PR : [URL_PR]
  2. Ajoute un commentaire avec ce texte exact :
     @copilot review
  3. Dans le sélecteur de modèle (bas gauche de la boîte), choisis : GPT-5.4

  ⚠️  Si tu ne fais rien → Copilot choisit automatiquement le modèle (auto-sélection).
```

Si l'utilisateur ne répond pas dans les 2 minutes → continuer avec la review automatique.

### Étape 6 — Lancer le Copilot Loop automatiquement

**Immédiatement après `gh pr create`**, invoquer le skill `copilot-loop` pour activer le polling via `ScheduleWakeup`.
Ne pas attendre que l'utilisateur le demande — c'est automatique.

Le loop se chargera de :
- Surveiller la review Copilot
- Intégrer la réponse dans le handoff JSON
- Merger dans la branche cible si `auto_merge_after_review = true`

Annoncer : "PR créée : [URL]. Loop Copilot activé — je surveille et intègrerai automatiquement."

**Ne pas afficher de prompt copier-coller. Copilot lit directement la PR GitHub.**

## Règles

- Format `.json` obligatoire — le markdown n'est pas reviewé par Copilot PR review
- Toujours passer par une branche `handoff/YYYY-MM-DD-slug` — jamais commit direct sur main
- Un handoff par review (pas de méga-fichier)
- La PR doit mentionner le fichier JSON et la question dans le body
- Committer le handoff avant de créer la PR (Copilot a besoin du fichier dans le diff)
