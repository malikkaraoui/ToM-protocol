---
name: atelier-setup
description: "Onboarding interactif post-install. Vérifie la config, guide le setup du watchdog et du review-reminder, propose BMAD et QMD. Utiliser après 'npx claude-atelier init' ou quand l'utilisateur dit /atelier-setup."
---

# Atelier Setup

Tu guides l'utilisateur dans la configuration complète de son atelier.
Checklist interactive — chaque point est vérifié puis coché.

## Checklist

Vérifie chaque point dans l'ordre. Pour chaque point :
- Si OK → affiche `[✅]` et passe au suivant
- Si KO → affiche `[❌]` avec les instructions pour corriger, attends
  que l'utilisateur confirme avant de continuer

### 1. Fichiers de base

```text
[?] .claude/CLAUDE.md installé et ≤ 150 lignes
[?] .claude/settings.json avec permissions Bash(*) + deny list
[?] .claudeignore à la racine
[?] .gitignore à la racine
[?] scripts/pre-push-gate.sh installé et exécutable
```

Si un fichier manque → proposer `npx claude-atelier init` ou
`node bin/cli.js init` si dans le repo source.

### 2. Contexte projet (§0)

Lire `.claude/CLAUDE.md` et vérifier que §0 est rempli :

```text
[?] Projet courant ≠ "—"
[?] Stack définie
[?] Repo défini
```

Si §0 est partiellement ou totalement vide → **auto-découvrir** :

**Nom du projet :**
```bash
cat package.json 2>/dev/null | python3 -c "import sys,json; print(json.load(sys.stdin).get('name',''))" 2>/dev/null || basename "$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
```

**Stack (en parallèle)** — détecte la présence de :
- `go.mod` → Go
- `package.json` → Node.js / JavaScript
- `Cargo.toml` → Rust
- `pyproject.toml` ou `requirements.txt` → Python
- `*.swift` → iOS
- `pom.xml` ou `build.gradle` → Java

```bash
detect_stack() {
  for f in go.mod package.json Cargo.toml pyproject.toml requirements.txt pom.xml build.gradle; do
    [ -f "$f" ] && case "$f" in
      go.mod) echo "Go"; return;;
      package.json) echo "Node.js"; return;;
      Cargo.toml) echo "Rust"; return;;
      pyproject.toml|requirements.txt) echo "Python"; return;;
      pom.xml|build.gradle) echo "Java"; return;;
    esac
  done
  compgen -G "*.swift" > /dev/null 2>&1 && echo "iOS" && return
  echo ""
}
detect_stack
```

**Repo GitHub :**
```bash
git remote get-url origin 2>/dev/null
```

**QMD enrichment** (optionnel) — si `.qmd/` existe ou `qmd status` fonctionne :
```bash
qmd query --searches '[{"type":"lex","query":"project stack technology"}]' 2>/dev/null || true
```

**Affichage et validation :**

Afficher les résultats avec `[AUTO]` :
```text
[AUTO] Projet : {nom}
[AUTO] Stack : {stack}
[AUTO] Repo : {url}
```

Demander :
"Valide ou corrige les découvertes avant que je les écrive dans §0.
Tape [OK] pour valider, ou corrige et tape [OK]."

Attendre réponse, puis éditer §0 avec les valeurs validées.

**Champs manquants :** Seulement si un champ reste vide après tout ça →
demander ce seul champ :
"Le champ `{nom}` reste vide. Donne-moi la valeur :"

### 3. Pouls Peter — agent mainteneur vault

```text
[?] vault/.peter/state.json présent (Peter actif)
```

Vérifier :
```bash
[ -f vault/.peter/state.json ] && echo "ACTIF" || echo "ABSENT"
```

Si absent → proposer :
"Lance `claude-atelier vault update` pour initialiser Peter. Il tiendra le vault à jour en autonomie via son pouls (`CronCreate`)."

> Le Night Watchdog manuel (Claude desktop → tâche planifiée) est **remplacé par le Pouls Peter** — cycle autonome géré par `CronCreate`, session-independent.

### 4. Séréna — Design Senior + MCP magic (optionnel)

```text
[ ] Skill design-senior installé
[ ] MCP magic (21st.dev) configuré
```

Vérifier :
```bash
ls .claude/skills/design-senior/ 2>/dev/null && echo "OK" || echo "MISSING"
grep -q "magic" ~/.claude.json 2>/dev/null || grep -q "magic" .mcp.json 2>/dev/null && echo "OK" || echo "MISSING"
```

Si skill absent → il sera copié au prochain `npx claude-atelier init`.

Si MCP magic absent → proposer :

"Séréna est la chef designer senior de l'atelier.
Elle s'active automatiquement quand tu parles design, UI/UX, charte.

Pour les composants UI premium, elle utilise le MCP **magic** (21st.dev).
L'installation est gratuite, l'utilisation nécessite une clé API
(free tier = 100 uses).

**Installation rapide (scope user, tous les projets) :**
```
claude mcp add magic --scope user --env API_KEY=\"ta-clé\" -- npx -y @21st-dev/magic@latest
```

1. Créer un compte : https://21st.dev
2. Générer une clé : https://21st.dev/agents/api-keys
3. Lancer la commande ci-dessus avec ta clé

[OUI] Installer magic | [SKIP] Continuer sans"

### 5. BMAD-METHOD (optionnel)

```text
[ ] BMAD-METHOD pour les gros projets
```

"Ce projet est-il un **gros projet** nécessitant un cycle complet
analyse → plan → architecture → implémentation ?

BMAD-METHOD fournit 6 agents spécialisés (analyste, PM, architecte,
dev, UX, tech writer) et un workflow structuré en 4 phases.

⚠️ C'est une méthodologie conséquente. Pour un petit projet ou un
script, ce n'est pas nécessaire.

[OUI] Installer BMAD | [NON] Continuer sans"

Si oui → `npx bmad-method install` dans le projet.

### 6. QMD (optionnel, conditionnel)

Compter les fichiers `.md` dans le projet :
`find . -name '*.md' -not -path './.git/*' -not -path './node_modules/*' | wc -l`

Si < 5 → ne pas proposer, passer.
Si ≥ 5 → proposer :

"Tu as **[N] fichiers markdown** dans ce projet. QMD peut les indexer
pour retrouver du contexte rapidement (plans, bugs, reviews).

[OUI] Installer QMD | [NON] Pas maintenant"

Si oui → guider l'installation de QMD (voir `src/fr/ecosystem/qmd-integration.md`).

### 7. Résumé

Afficher le résumé final :

```text
╔══════════════════════════════════════════════════╗
║  🔧 Atelier configuré !                          ║
╠══════════════════════════════════════════════════╣
║  [✅] Config de base (5/5)                       ║
║  [✅] Contexte projet §0 rempli                  ║
║  [✅] Pouls Peter actif (vault autonome)         ║
║  [✅] Séréna + MCP magic configurés             ║
║  [—]  BMAD : non installé (petit projet)        ║
║  [—]  QMD : non installé (< 5 fichiers .md)     ║
╠══════════════════════════════════════════════════╣
║  Tape /atelier-help pour voir les commandes.    ║
╚══════════════════════════════════════════════════╝
```

## Règles

- Toujours vérifier avant de corriger
- Ne jamais forcer une installation (proposer, pas imposer)
- BMAD et QMD sont **optionnels** — ne pas insister
- Respecter les choix de l'utilisateur (SKIP est valide)
