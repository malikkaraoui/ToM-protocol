Lance `claude-atelier features` et affiche le tableau de contrôle.

**AVANT** d'afficher le tableau, exécute ces 3 commandes en parallèle pour construire le bloc état système :

```bash
# Proxy :4000
lsof -i :4000 -sTCP:LISTEN 2>/dev/null | grep -c LISTEN

# Ollama installé + modèles
which ollama 2>/dev/null && ollama list 2>/dev/null || echo "OLLAMA_ABSENT"

# Modèle Claude actif — extrait du contexte [ROUTING] modèle actif: MODEL-ID
# (pas de commande bash : lire la valeur dans les hooks de la session courante)
```

Puis affiche ce bloc en tête de réponse (avant le tableau features) :

```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  État système
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  Claude actif   : <MODEL-ID issu de [ROUTING] modèle actif:>
  Proxy :4000    : ✅ actif  OU  ❌ inactif  (selon lsof)
  Ollama         : ✅ installé (<N> modèles)  OU  ❌ non installé — /ollama-router pour setup
                   Si installé → liste les modèles (1 par ligne, format compact)
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

Si l'utilisateur demande à modifier une feature ou un paramètre, exécute la commande correspondante :

**Features (on/off) :**
- Activer   : `npx claude-atelier features --on <feature>`
- Désactiver : `npx claude-atelier features --off <feature>`
- Basculer  : `npx claude-atelier features --toggle <feature>`

**Paramètres configurables (valeurs numériques) :**
- Modifier : `npx claude-atelier features --set <param> <valeur>`
- Exemples de params : `handoff_threshold_lines`, `context_warning_kb`, `context_critical_kb`, `anti_loop_count`, `diagnostic_interval_min`

**Réinitialiser tout :**
- `npx claude-atelier features --reset`

Après chaque modification, rappelle que Claude Code doit être relancé pour appliquer le changement.

---

Ensuite, affiche **toujours** le tableau des agents disponibles :

| Commande | Agent | Rôle |
|---|---|---|
| `/atelier-help` | Atelier Help | État du projet + commandes disponibles |
| `/atelier-setup` | Atelier Setup | Onboarding post-install, setup watchdog & QMD |
| `/atelier-doctor` | Atelier Doctor | Diagnostic complet installation (27+ checks) |
| `/atelier-config` | Atelier Config | Ce tableau de contrôle |
| `/review-copilot` | Review Copilot | Génère un handoff review pour Copilot/GPT (§25) |
| `/integrate-review` | Integrate Review | Intègre la réponse Copilot depuis docs/handoffs/ |
| `/copilot-loop` | Copilot Loop | Loop autonome PR→review→merge |
| `/la-bise` | La Bise | Échange inter-LLM (GPT/Mistral) |
| `/angle-mort` | Angle Mort | Review anti-complaisance avant release |
| `/compress` | Compress | Compresse CLAUDE.md pour réduire les tokens |
| `/audit-safe` | Audit Safe | Scan secrets, gate, permissions, .claudeignore |
| `/night-launch` | Night Launch | Prépare le mode nuit (autonomie) |
| `/token-routing` | Token Routing | Configure le routing Haiku/Sonnet/Opus |
| `/design-senior` | Design Senior | Propose Séréna + installe UI/UX Pro Max |
| `/bmad-init` | BMAD Init | Installe BMAD-METHOD dans le projet |
| `/qmd-init` | QMD Init | Installe QMD (moteur recherche .md local) |
| `/ollama-router` | Ollama Router | Setup Ollama bout-en-bout + proxy |
| `/ios-setup` | iOS Setup | Workflow iOS/tvOS : VS Code + Xcode + Makefile |
| `/freebox-init` | Freebox Init | Bootstrap autorisation app Freebox |
| `/handoff-debt` | Handoff Debt | Affiche la dette §25 + draft handoff |

---

**⚠️ Note entête §1** : Les flags `header_show_*` sont **modifiables via CLI** (`--on/--off/--toggle`) et écrits dans `features.json`, mais **aucun hook ne les lit encore** — ils n'ont donc pas d'effet runtime pour l'instant. Le câblage dans les hooks §1 est un chantier séparé. Ne pas modifier le comportement réel de l'entête sans instruction explicite.
