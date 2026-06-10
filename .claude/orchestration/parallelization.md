---
kind: orchestration
name: parallelization
loads_from: src/fr/CLAUDE.md §16
replaces: src/fr/orchestration/_legacy.md (partiel)
---

# Orchestration — Parallélisation

> Chargé à la demande. Les patterns ci-dessous transforment Claude d'un
> assistant séquentiel en une **équipe de spécialistes** qui travaillent
> simultanément. Chaque pattern est un scénario réel, testé, documenté.

---

## Pattern 1 — Parallel Audit Storm

### Scénario

Tu viens de finir une feature de 200 lignes. Il est 23h, tu veux
lancer le night-mode et aller dormir. Sauf qu'avant, il faut vérifier :
secrets ? lint ? refs markdown ? tests ? Séquentiellement, ça prend
12-15 minutes. Tu bâilles devant l'écran en attendant que chaque étape
finisse avant de lancer la suivante.

**Avec le Parallel Audit Storm** : tu lances 4 agents en un seul
message. Chacun est un spécialiste. Ils partent tous en même temps.
3 minutes plus tard, les 4 résultats atterrissent. Claude parent
synthétise tout en un verdict unique : vert ou rouge, avec les
détails de chaque agent.

Tu fermes ton laptop. L'audit complet a pris 3 minutes au lieu de 15.

### Architecture de l'audit

```text
Toi : "Audit complet avant night-mode"

Claude parent ──┬── Agent 1 (Haiku) : grep secrets patterns
                ├── Agent 2 (Haiku) : npm run lint
                ├── Agent 3 (Haiku) : lint refs markdown
                └── Agent 4 (Haiku) : npm test

        ↓ 3 min ↓

Claude parent : "4/4 vert. Prêt pour le night-mode."
         ou   : "3/4 vert. Agent 2 a trouvé : [détail]. Tu fixes ?"
```

### Prompts des 4 agents

Claude spawne les 4 agents **dans un seul message** (c'est la clé —
un seul message = exécution parallèle, pas séquentielle) :

```text
Agent 1 — Secrets scan
  subagent_type: Explore
  model: haiku
  prompt: "Cherche dans tout le repo les patterns de secrets :
           sk-, AKIA, ghp_, AIza, password=, token=.
           Rapporte chaque occurrence avec fichier:ligne."

Agent 2 — Lint
  subagent_type: general-purpose
  model: haiku
  prompt: "Lance npm run lint dans [REPO]. Rapporte OK ou les erreurs."

Agent 3 — Refs markdown
  subagent_type: Explore
  model: haiku
  prompt: "Vérifie que chaque lien relatif dans les .md du repo
           pointe vers un fichier existant. Liste les liens cassés."

Agent 4 — Tests
  subagent_type: general-purpose
  model: haiku
  prompt: "Lance npm test dans [REPO]. Rapporte OK ou les échecs."
```

### Coût

4 agents Haiku en parallèle coûtent **moins cher qu'un seul Sonnet
séquentiel** sur la même durée. Le routing modèle fait la différence :
la tâche est mécanique (grep, lint, test), Haiku suffit largement.

### Quand l'utiliser

- Avant un night-mode (audit de sécurité complet)
- Avant un push (alternative rapide à la gate séquentielle)
- Après un refactor large (vérification multi-axe)
- Sur demande : « audit storm » ou « vérifie tout »

---

## Pattern 2 — Background Copilot Review

### Scénario — le flow interrompu

Tu viens de finir d'implémenter un système d'authentification. 150
lignes. Tu sais que tu dois envoyer un handoff à Copilot pour une
review croisée (§25 — le réflexe). Mais tu es dans le flow. La
prochaine feature t'attend. Si tu t'arrêtes pour écrire le handoff,
tu perds le momentum. Si tu ne l'écris pas, tu oublies.

**Avec le Background Copilot Review** : tu dis « prépare un handoff
review en background ». Claude spawne un agent en arrière-plan qui
lit tes derniers commits, analyse le diff, écrit le handoff structuré
dans `docs/handoffs/`, et te prévient quand c'est prêt.

Pendant ce temps, toi, tu continues à coder la feature suivante.
Zero interruption. Zero oubli.

### Architecture du background review

```text
Toi : "Handoff Copilot en background pour l'auth"
                                                    ┌─────────────────────────┐
Claude : "Je lance ça en background."               │  Agent background       │
         ↓ continue à coder avec toi ↓              │  1. git log --oneline   │
                                                     │  2. git diff HEAD~5     │
Toi : "Ok, maintenant le module de paiement..."     │  3. Analyse les changes │
Claude : "J'implémente le paiement."                │  4. Rédige le handoff   │
                                                     │  5. Écrit dans          │
         ↓ 5 min passent ↓                          │     docs/handoffs/      │
                                                     └──────────┬──────────────┘
Claude : "Le handoff auth est prêt :                            │
  docs/handoffs/2026-04-12-review-auth.md            ←──────────┘
  Tu peux le coller dans Copilot quand tu veux."

Toi : (sans avoir quitté ton flow une seule seconde)
```

### Prompt de l'agent background

```text
Agent — Background Copilot Handoff
  subagent_type: general-purpose
  run_in_background: true
  prompt: "Tu prépares un handoff review pour Copilot/GPT.
           1. Lis les 5 derniers commits avec git log -5 --stat
           2. Lis le diff complet avec git diff HEAD~5
           3. Identifie le sujet principal (feature, bug fix, refactor)
           4. Écris un handoff structuré dans docs/handoffs/YYYY-MM-DD-review-[sujet].md
              avec les sections : Contexte, Question précise, Fichiers à lire,
              Contraintes / hors scope
           5. Termine avec la section vide 'Réponse de : Copilot/GPT'
           Ne modifie aucun autre fichier. Ne commite pas."
```

L'agent tourne **en arrière-plan**. Quand il finit, Claude reçoit
une notification et l'annonce. Si l'agent a besoin d'une permission,
le watchdog Cowork peut cliquer Allow automatiquement (night-mode).

### Le multiplicateur

Ce pattern se combine avec le Parallel Audit Storm. Imagine :

```text
"Fais un audit storm + handoff review en background"

Claude ──┬── Agent 1-4 : audit parallèle (foreground, résultat immédiat)
         └── Agent 5   : handoff review (background, notification quand prêt)
```

5 agents. 3 minutes. Un audit complet + un handoff prêt à envoyer.
Toi, tu as continué à coder pendant tout ce temps.

### Quand l'utiliser

- Feature terminée + prochaine feature qui attend (zero interruption)
- Fin de session : « prépare un handoff de tout ce qu'on a fait »
- Avant de partir en review : le handoff est déjà écrit quand tu y arrives
- §25 automatic trigger : 100+ lignes modifiées → proposer le background

---

## Pattern 3 — Multi-Session CLI

### Scénario — deux Claude, un projet

Ouvre deux terminaux côte à côte. Terminal gauche : Claude code ta
feature. Terminal droit : un autre Claude tourne en boucle et lance
les tests à chaque commit.

Tu ne fais rien. Tu regardes. Terminal gauche commit. Terminal droit
détecte le commit, lance les tests, affiche le résultat. Si c'est
rouge, terminal droit t'alerte. Si c'est vert, il attend le prochain
commit.

C'est comme avoir un développeur ET un QA qui bossent en temps réel,
sur la même machine, en même temps. Sauf que les deux sont Claude.

### Architecture temps réel

```text
┌─────────────────────────────┐  ┌─────────────────────────────┐
│  Terminal 1 — Dev           │  │  Terminal 2 — Tests continus │
│                             │  │                              │
│  $ claude                   │  │  $ claude --permission-mode  │
│  "Implémente le module X"   │  │    acceptEdits               │
│                              │  │    "Tu es un runner de tests │
│  → Code, commit, code...    │  │     continu. Toutes les 2    │
│  → Commit atomique          │  │     minutes, lance git log   │
│  → Continue la feature      │  │     -1. Si nouveau commit    │
│                              │  │     depuis le dernier check, │
│                              │  │     lance npm test. Rapporte │
│                              │  │     OK ou les échecs.        │
│                              │  │     Ne modifie aucun fichier.│
│                              │  │     Boucle indéfiniment."    │
│                              │  │                              │
│  → Commit #3                │  │  [check] nouveau commit #3   │
│                              │  │  [run] npm test...           │
│  → Continue...              │  │  [OK] 47/47 tests passed     │
│                              │  │  [wait] prochain commit...   │
│                              │  │                              │
│  → Commit #4                │  │  [check] nouveau commit #4   │
│                              │  │  [run] npm test...           │
│  → Continue...              │  │  [FAIL] auth.test.js:23      │
│                              │  │  → "Test échoué : expected   │
│                              │  │    200 got 401. Le commit #4 │
│                              │  │    a cassé l'auth."          │
│                              │  │                              │
│  "Ok je fix l'auth"         │  │  [wait] prochain commit...   │
└─────────────────────────────┘  └─────────────────────────────┘
```

### Le setup

Terminal 1 — Dev (normal) :

```bash
claude
# Tu travailles normalement. Commits atomiques.
```

Terminal 2 — Test runner continu :

```bash
claude --permission-mode acceptEdits \
  "Tu es un test runner continu pour le projet $(pwd).
   Toutes les 2 minutes :
   1. git log -1 --format='%H %s'
   2. Compare avec le dernier hash connu
   3. Si nouveau commit → lance npm test (ou pytest, ou mvn test)
   4. Rapporte : [OK] N/N tests passed ou [FAIL] fichier:ligne + message
   5. Ne modifie AUCUN fichier. Ne commite rien. Lecture seule.
   Boucle indéfiniment jusqu'à ce que je t'arrête."
```

### Variantes avancées

**3 terminaux** — Dev + Tests + Lint continu :

```text
Terminal 1 : Claude dev (feature)
Terminal 2 : Claude test runner (npm test à chaque commit)
Terminal 3 : Claude lint watcher (npm run lint à chaque commit)
```

**Continuous security scan** :

```bash
claude --permission-mode acceptEdits \
  "Tu es un scanner de sécurité continu. Toutes les 5 minutes,
   cherche les patterns de secrets (sk-, AKIA, ghp_, AIza) dans
   les fichiers modifiés depuis le dernier check. Alerte si trouvé.
   Ne modifie rien."
```

**Continuous doc sync** :

```bash
claude --permission-mode acceptEdits \
  "Tu es un vérificateur de documentation continu. Toutes les 5 min,
   vérifie que les liens markdown pointent vers des fichiers existants.
   Si un lien est cassé après un commit, alerte. Ne modifie rien."
```

### Pourquoi c'est puissant

Chaque terminal est une **session Claude indépendante**. Elles ne se
connaissent pas, ne partagent pas de contexte, et ne se marchent pas
dessus. C'est de l'isolation naturelle sans configuration.

Le terminal dev a le contexte complet de la feature. Le terminal test
ne fait que lire et exécuter. Pas de conflit, pas de coordination
compliquée, pas de setup. Juste deux `claude` dans deux terminaux.

Et le coût ? Le test runner attend 95% du temps (poll toutes les 2 min).
Il ne consomme des tokens que quand il détecte un nouveau commit et
lance les tests. C'est presque gratuit.

### Quand l'utiliser

- Night-mode supervisé : dev dans Terminal 1, watchdog dans Terminal 2
- Feature complexe avec risque de régression (tests continus)
- Refactor avec beaucoup de fichiers (lint + tests en parallèle)
- Journée de dev intensive (feedback loop permanent)

---

## Récap — Les 3 patterns en un coup d'oeil

| Pattern | Agents | Durée | Coût | Effet |
| --- | --- | --- | --- | --- |
| **Audit Storm** | 4 Haiku foreground | 3 min | < 1 Sonnet | Audit complet 4x plus rapide |
| **Background Review** | 1 background | 5 min (invisible) | 1 agent | Review sans interrompre le flow |
| **Multi-Session** | N terminaux | Continu | Poll = presque gratuit | CI locale temps réel |

---

## Quand NE PAS paralléliser

- **Tâches séquentielles** avec dépendances (A doit finir avant B)
- **Tâches < 2 min** : l'overhead de coordination > le gain
- **Budget token serré** : 3 agents en parallèle = 3x le coût d'un seul
- **Incertitude** : si on ne sait pas encore quoi faire, planifier d'abord
  (séquentiel), puis exécuter (potentiellement parallèle)

## Discipline

- **Maximum 3-4 agents parallèles** en pratique : au-delà, coordination
  et budget dérapent
- **Toujours assigner des fichiers distincts** à chaque agent
- **Un agent coordinator** (le parent) synthétise les résultats
- **Arrêter un agent dès qu'il a fini** : ne pas laisser idle

## Voir aussi

- `./modes.md` — Fork vs Teammate vs Worktree
- `./spawn-rules.md` — règles de prompt et limites par agent
- `./models-routing.md` — quel modèle pour quel rôle
