# Skill : /loop-master

> Orchestrateur de pipeline. Lance, boucle, arrête.
> Règle : Claude Code ne travaille JAMAIS seul sur une tâche code — `/loop-master` est obligatoire.

## Déclenchement

**Hook `guard-loop-master.sh` (PreToolUse `git commit`)** — bloque `git commit` si flag
`/tmp/claude-atelier-loop-done` absent sur une tâche de type feature/refactor/fix.

Ou déclenché manuellement : `/loop-master <TARGET>`

## Pipeline

```
[CHEF PROJET] → ouverture + estimation + GO
      ↓
[CODEUR]        → implémente les objectifs du sprint
      ↓
[RELECTEUR]     → code + sécu + npm test réel
      ↓
[DOCUMENTALISTE]→ met à jour vault Obsidian via MCP obsidian-vault
      ↓
[CHEF PROJET]   → TARGET atteinte ? NON → nouveau sprint | OUI → clore
```

## Paramètres

```
/loop-master [TARGET]
```

- `TARGET` : description libre, chemin vers un fichier markdown, ou `vault/40-roadmap.md`
- Sans argument : lit `vault/40-roadmap.md` et sélectionne le premier item "Sur le feu"

## Séquence d'exécution

```javascript
// Pseudo-code — exécuté via Agent tool
1. agent("Chef Projet — ouvrir sprint + estimer", {label: "Chef Projet — ouverture sprint"})
2. boucle (max 5) :
   a. agent("Codeur — [objectifs sprint]", {label: "Codeur — sprint N"})
   b. agent("Relecteur — code + sécu + npm test", {label: "Relecteur — sprint N"})
   c. agent("Documentaliste — vault MAJ", {label: "Documentaliste — sprint N"})
   d. agent("Chef Projet — TARGET atteinte ?", {label: "Chef Projet — boucle N/5"})
      → si OUI → break
      → si NON → préparer sprint suivant → continuer
3. touch /tmp/claude-atelier-loop-done
4. agent("Chef Projet — clôture + vault/50-token-history", {label: "Chef Projet — clôture"})
```

## Contrat sous-agents

| Agent | Lecture | Écriture |
|-------|---------|---------|
| Codeur | tout | code source uniquement |
| Relecteur | tout | aucune (rapport uniquement) |
| Documentaliste | vault/ | vault/ via MCP obsidian-vault |
| Chef Projet | tout | vault/50-token-history.md |

## Métriques inter-agents

Chaque sous-agent écrit dans `/tmp/claude-atelier-loop-metrics.json` :
```json
{
  "sprint": 1,
  "agent": "codeur",
  "tokensUsed": 12400,
  "status": "done",
  "findings": []
}
```
Chef de Projet lit ce fichier pour le monitoring et la clôture.

## Règles

- Max 5 boucles — au-delà : escalader à l'utilisateur
- `npm test` DOIT passer avant clôture (Relecteur le vérifie)
- Documentaliste écrit via MCP obsidian-vault (jamais via fichier direct)
- `vault/50-token-history.md` mis à jour à chaque fin de phase
- Flag `/tmp/claude-atelier-loop-done` posé uniquement si TARGET atteinte ET tests verts
