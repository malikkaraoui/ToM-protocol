---
name: qmd-init
description: "Installe et configure QMD (moteur de recherche markdown local). Proposé automatiquement par /atelier-setup si le projet contient >= 5 fichiers .md."
---

# QMD Init

Tu installes QMD dans le projet courant pour indexer les fichiers
markdown (plans, bug reports, handoffs, reviews, specs).

## Quand proposer

- Le projet contient ≥ 5 fichiers `.md`
- L'utilisateur travaille avec des handoffs inter-LLM (docs/handoffs/)
- Pipeline Claude ↔ Copilot actif (les fichiers s'accumulent vite)
- Projet BMAD en cours (beaucoup d'artefacts .md)

## Procédure

### Étape 1 — Vérifier les prérequis

```bash
node --version   # doit être >= 22
```

### Étape 2 — Installer QMD

```bash
npm install -g @tobilu/qmd
qmd --version
```

### Étape 3 — Créer la collection

```bash
qmd collection add . --name [nom-projet] --mask "**/*.md"
qmd context add qmd://[nom-projet] "Plans, specs, handoffs, reviews, bug reports"
```

### Étape 4 — Indexer

```bash
qmd embed
qmd status
```

### Étape 5 — Configurer le MCP

Ajouter dans `.claude/settings.json` :

```json
{
  "mcpServers": {
    "qmd": {
      "command": "qmd",
      "args": ["mcp"]
    }
  }
}
```

Mettre à jour §0 de CLAUDE.md : `MCPs actifs | qmd`.

### Étape 6 — Test

```bash
qmd query "test" --files -n 3
```

Si des résultats → QMD fonctionne.

"QMD installé et indexé. [N] fichiers markdown indexés.
Pour chercher : `qmd query 'ton sujet' --files`
Guide complet : docs/qmd-user-guide.md"

## Règles

- Ne proposer que si ≥ 5 fichiers .md dans le projet
- Ne pas forcer l'installation
- Guide complet → `docs/qmd-user-guide.md` (déjà dans le repo)
- Voir aussi `src/fr/ecosystem/qmd-integration.md` pour les règles
  d'utilisation par Claude

## Après l'installation

L'indexation peut prendre 30-60 secondes et faire tourner le ventilateur
(téléchargement et calcul des embeddings localement). C'est normal —
les embeddings sont calculés localement.

**Important — redémarrage obligatoire :**

Redémarrer le chat dans VS Code **ne suffit pas**. Le MCP est chargé
au démarrage du **processus Claude Code**, pas du chat.

Pour forcer le rechargement des MCP servers :

- **Option A (VS Code)** : `Cmd+Shift+P` → `Claude Code: Restart`
  (ou `Claude Code: Restart Server`) → tue et relance le processus
- **Option B** : fermer et rouvrir VS Code complètement

QMD via Bash (`qmd query "..."`) fonctionne immédiatement sans redémarrage.
C'est le MCP natif (outil intégré directement dans Claude) qui nécessite
le redémarrage.

"QMD installé. ⚠️ Redémarre le processus Claude Code (Cmd+Shift+P → Claude Code: Restart) pour activer le MCP. En attendant, `qmd query` via Bash fonctionne déjà."
