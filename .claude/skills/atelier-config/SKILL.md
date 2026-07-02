---
name: atelier-config
description: "Tableau de contrôle des features claude-atelier (on/off, paramètres). Affiche l'état de chaque rail et permet de les activer/désactiver interactivement."
figure: Régie
---

# Atelier Config — Tableau de contrôle interactif

> La régie. Tous les rails sous les yeux. On allume, on éteint, on règle.

## Procédure

### 1. Lire features.json

Chemin : `.claude/features.json` (racine du repo).

Si le fichier est absent ou vide `{}`, créer avec ces defaults avant de continuer :
```json
{
  "header": true,
  "askUserQuestion": {
    "enabled": true,
    "triggers": ["vault-ingest", "review-copilot", "model-switch"]
  }
}
```

### 2. Afficher la config actuelle

Extraire et afficher le tableau suivant (valeurs réelles depuis features.json, défaut si absente) :

| Feature | Valeur | Défaut |
|---|---|---|
| `header` | ✅ ON / ❌ OFF | `true` |
| `askUserQuestion` | ✅ ON / ❌ OFF | `true` |

### 3. Poser la question via AskUserQuestion

Appeler l'outil **AskUserQuestion** (natif Claude Code — ne pas créer de wrapper) :

- **question** : `"Que voulez-vous modifier ?"`
- **options** :
  1. `"Toggle askUserQuestion — activer/désactiver les questions interactives"`
  2. `"Toggle header — activer/désactiver l'en-tête §1"`
  3. `"Quitter sans modification"`

### 4. Appliquer la modification

**Option 3 — Quitter** : répondre `"Aucune modification."` et s'arrêter.

**Options 1–2 — Toggle** :
- Lire la valeur actuelle dans features.json (défaut si clé absente)
- Inverser la valeur :
  - `askUserQuestion` → inverser `askUserQuestion.enabled` (bool)
  - `header` → inverser la valeur bool de `header`
- Écrire avec **Edit** (jamais réécriture complète si d'autres clés existent)
- Confirmer en une ligne : `` ✅ `<feature>` → `<nouvelle valeur>` ``

### 5. Commit atomique

Après toute modification de features.json :
```bash
git add .claude/features.json
git commit -m "config: toggle <feature>"
```

## Règles strictes

- Pas de commentaires dans features.json — JSON strict
- Si features.json absent → le créer avec defaults, puis appliquer la modification
- AskUserQuestion = outil natif, pas de wrapper shell ni script intermédiaire
- Un seul toggle par appel — relancer `/atelier-config` si plusieurs changements

## Agents disponibles — commandes slash

| Commande | Agent | Rôle |
|---|---|---|
| `/atelier-help` | Atelier Help | État du projet + commandes disponibles |
| `/atelier-setup` | Atelier Setup | Onboarding post-install, setup watchdog & QMD |
| `/atelier-doctor` | Atelier Doctor | Diagnostic complet installation (27+ checks) |
| `/atelier-config` | Atelier Config | Ce tableau de contrôle |
| `/review-copilot` | Review Copilot | Génère un handoff review pour Copilot/GPT (§25) |
| `/integrate-review` | Integrate Review | Intègre la réponse Copilot depuis docs/handoffs/ |
| `/la-bise` | La Bise | Échange inter-LLM (GPT/Mistral) |
| `/angle-mort` | Angle Mort | Review anti-complaisance avant release |
| `/compress` | Compress | Compresse CLAUDE.md pour réduire les tokens |
| `/audit-safe` | Audit Safe | Scan secrets, gate, permissions, .claudeignore |
| `/night-launch` | Night Launch | Prépare le mode nuit (autonomie) |
| `/token-routing` | Token Routing | Configure le routing Haiku/Sonnet/Opus |
| `/design-senior` | Design Senior | Propose Séréna + installe UI/UX Pro Max |
| `/bmad-init` | BMAD Init | Installe BMAD-METHOD dans le projet |
| `/qmd-init` | QMD Init | Installe QMD (moteur recherche .md local) |
| `/ios-setup` | iOS Setup | Workflow iOS/tvOS : VS Code + Xcode + Makefile |
| `/freebox-init` | Freebox Init | Bootstrap autorisation app Freebox |
| `/handoff-debt` | Handoff Debt | Affiche la dette §25 + draft handoff |