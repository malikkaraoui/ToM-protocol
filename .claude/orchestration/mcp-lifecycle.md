---
kind: orchestration
name: mcp-lifecycle
loads_from: src/fr/CLAUDE.md §19
replaces: src/fr/orchestration/_legacy.md (partiel)
---

# Orchestration — Cycle de vie des MCPs

> Chargé à la demande. Règles de chargement, suivi et purge des
> MCP servers dans une session Claude Code.

## Concept

Un **MCP server** (Model Context Protocol) expose des outils et
des ressources à Claude via un protocole standardisé (stdio ou HTTP).
Chaque MCP chargé consomme de la fenêtre de contexte : descriptions
d'outils, instructions, schémas.

## Impact sur la fenêtre de contexte

- **Fenêtre nominale** : ~200k tokens
- **Avec beaucoup de MCPs** : fenêtre effective **~70k tokens**
- Chaque MCP ajoute : descriptions d'outils, instructions de serveur,
  schémas de paramètres, résultats d'appels

C'est le principal coût caché de la productivité en Claude Code :
plus d'outils ≠ plus de capacité.

## Règles de gestion

### Chargement

- **Charger uniquement les MCPs nécessaires à la session courante**
- **Lister les MCPs actifs dans §0 de CLAUDE.md** (champ « MCPs actifs »)
- Si un MCP n'est pas utilisé dans les 10 premières minutes, il n'a
  probablement rien à faire dans cette session

### Purge

- **En fin de session** : purger les MCPs non persistants
- **En fin de projet** : retirer les MCPs spécifiques au projet de
  `settings.json` global
- **Au changement de phase** : réévaluer si les MCPs de la phase
  précédente sont encore nécessaires

### Configuration

MCPs dans `settings.json` (global ou projet) :

```json
{
  "mcpServers": {
    "qmd": {
      "command": "qmd",
      "args": ["mcp"]
    },
    "mon-outil": {
      "command": "npx",
      "args": ["-y", "@scope/mon-outil-mcp"]
    }
  }
}
```

## Anti-patterns

- **Tous les MCPs tout le temps** : 10 MCPs chargés = fenêtre réduite à
  30–40 % d'utile. Charger strictement ce qui sert.
- **MCP oublié après un projet** : persiste dans settings.json global,
  consomme à chaque session suivante
- **MCP en double** : plugin qui installe un MCP + config manuelle du même
  → deux instances, double coût
- **MCP daemon non arrêté** : `qmd mcp --http --daemon` par exemple,
  qui reste actif après la session (vérifier `qmd status`)

## Voir aussi

- `../ecosystem/plugins.md` — les plugins installent souvent des MCPs
- `../ecosystem/qmd-integration.md` — MCP QMD spécifique
