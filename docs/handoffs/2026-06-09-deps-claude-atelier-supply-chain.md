# Handoff — Deps : claude-atelier + sécurité supply chain

> Date : 2026-06-09
> Type : review
> Priorité : moyenne (devDep uniquement, pas en runtime)
> reviewedRange: 1d38544..245a927

---

## De : Claude (Sonnet 4.6)

### Contexte

Commits `1d38544` + `245a927` — `chore(deps)` + `fix(deps)`

`claude-atelier` (github:malikkaraoui/claude-atelier) a été ajouté en
`devDependency` pour synchroniser les skills, hooks et stacks Claude Code dans
le projet. Un scanner de sécurité automatique a immédiatement signalé le risque
supply chain d'une dépendance non épinglée depuis un compte GitHub personnel.

Correction appliquée dans `245a927` : spécifier épinglé au commit exact
`#618aef21902bd0e92ec48d5aedf2ebc842c0eb68` dans `package.json`.

`pnpm-lock.yaml` verrouille la résolution via codeload.github.com (tarball SHA).

Sous-dépendances notables introduites : `better-sqlite3@12.9.0` (module natif,
nécessite compilation C++ à l'install), `bindings`, `bl`.

### Fichiers à lire

- `package.json` — ligne `claude-atelier` (doit avoir le hash #618aef2...)
- `pnpm-lock.yaml` — entrée `claude-atelier` + `better-sqlite3`

### Question précise

1. **Risque résiduel du pin** : épingler `#hash` dans `package.json` empêche
   un upgrade silencieux, mais si le repo upstream est rewrite ou force-pushed,
   le hash peut pointer vers du code différent. Est-ce suffisant pour une
   devDep, ou faut-il aller jusqu'au vendoring ?

2. **better-sqlite3 en CI** : `pnpm install --frozen-lockfile` va compiler
   `better-sqlite3` sur `ubuntu-latest` (CI GitHub). Le job `typescript` n'a
   pas `python3`/`make` explicitement installés — sont-ils disponibles sur
   `ubuntu-latest` par défaut ? Y a-t-il un risque d'échec CI sur ce point ?

3. **Lifecycle scripts** : `better-sqlite3` a un `install` script (`node-pre-gyp`).
   pnpm 9.x autorise les lifecycle scripts par défaut. Le scanner suggère
   `neverBuiltDependencies` pour bloquer l'exécution de scripts à l'install.
   Faut-il l'ajouter pour limiter la surface d'attaque ?

4. **Alternative** : le scanner suggère d'installer `claude-atelier` globalement
   via `pnpm dlx` plutôt que dans le workspace. Compte tenu de l'usage
   (skills Claude Code, pas du code applicatif), est-ce préférable ?

### Contraintes / hors scope

- `claude-atelier` est un outil de développement, jamais en runtime production
- Le repo `malikkaraoui/claude-atelier` appartient au même développeur (Malik)
- Ne pas proposer de migrer vers npm registry (dépendance privée/perso)

---

## Réponse de : Copilot/GPT

**INSTRUCTION POUR LE LLM QUI RÉPOND :**
Tu DOIS écrire ta réponse directement dans **ce même fichier `.md`**, à la
suite du document existant, dans la section `## Réponse de : Copilot/GPT`.
Tu ne dois PAS répondre dans le chat, ni dans le prompt, ni dans un autre fichier.
Ce document est le vase communicant entre GPT et Claude. Utilise Edit/Write.
Quand tu as fini, dis : "J'ai répondu dans `docs/handoffs/2026-06-09-deps-claude-atelier-supply-chain.md`."

---

## Intégration
<!-- Claude remplit après lecture de la réponse -->
