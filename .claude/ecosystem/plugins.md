---
kind: ecosystem
name: plugins
loads_from: src/fr/CLAUDE.md (header + §19)
volatile: true
---

# Ecosystem — Plugins

> Chargé à la demande. Explique l'installation, l'usage et la discipline
> d'utilisation des plugins Claude Code depuis un marketplace.

## Concept

Un **plugin** est un bundle de capacités (skills, agents, hooks, MCP servers,
slash commands) distribué via un marketplace Git ou un registry officiel.

Installation typique :

```bash
# Ajouter un marketplace
/plugin marketplace add <owner>/<repo>

# Installer un plugin
/plugin install <plugin-name>

# Lister les plugins actifs
/plugin list

# Desinstaller
/plugin uninstall <plugin-name>
```

## Quand installer un plugin

- **Besoin récurrent bien défini** : le plugin automatise une tâche qu'on
  refait souvent (ex: `vercel-plugin` pour tout projet Vercel)
- **Connaissance domaine à jour** : le plugin apporte des docs fraîches
  que la training data n'a pas (Next.js récent, Vercel AI SDK…)
- **Outillage intégré** : le plugin fournit des MCP servers ou des hooks
  qu'il serait coûteux de câbler à la main

## Quand NE PAS installer un plugin

- **Usage unique** : un plugin qui sert une seule fois n'apporte rien
- **Stack instable** : installer 20 plugins fragmente la fenêtre de contexte
  (rappel : trop de MCPs → 200k devient ~70k utiles)
- **Duplication** : deux plugins qui couvrent le même domaine créent des
  suggestions contradictoires

## Discipline de gestion

- **Lister les plugins actifs dans `§0 CLAUDE.md`** (champ « MCPs actifs »)
- **Purger régulièrement** : en fin de projet ou de phase, retirer les plugins
  qui ne servent plus
- **Vérifier les hooks** : un plugin installe souvent des hooks `SessionStart`,
  `UserPromptSubmit`, `PreToolUse` — ils peuvent injecter du contexte
  involontairement (cf. `./hooks.md`)

## Comprendre les suggestions de plugins

Les plugins actifs peuvent injecter des messages `<system-reminder>` :

- **MANDATORY** : à traiter sérieusement, mais **vérifier la pertinence**
  (faux positifs lexicaux fréquents)
- **Best practices auto-suggested** : orientation, pas une obligation
- **Official documentation** : signale que la training data est potentiellement
  obsolète sur ce domaine — charger la doc fraîche avant d'écrire du code

## Anti-patterns

- Installer un plugin « au cas où » sans avoir un besoin identifié
- Ignorer systématiquement toutes les suggestions d'un plugin (si tu
  l'ignores toujours, désinstalle-le)
- Accepter aveuglément toutes les suggestions sans contexte (saturation
  de la fenêtre, dérive du raisonnement)
