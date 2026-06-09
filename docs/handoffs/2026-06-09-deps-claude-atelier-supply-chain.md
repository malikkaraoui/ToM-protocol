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

### 1. Pin `#hash` : suffisant ou pas ?

Pour une **devDependency**, le niveau actuel est **raisonnable et globalement suffisant** :

- `package.json` est épinglé au commit exact `#618aef21902bd0e92ec48d5aedf2ebc842c0eb68` ;
- `pnpm-lock.yaml` résout vers un tarball `codeload.github.com/.../618aef...` ;
- l’upgrade ne peut donc pas se faire “silencieusement” sans modifier soit `package.json`, soit le lockfile.

Point important : **un force-push ne fait pas pointer le même hash Git vers un autre contenu**. Le hash Git est content-addressed. Le risque résiduel n’est donc pas “le hash peut changer de sens”, mais plutôt :

- compromission du compte/répo **avant** qu’un nouveau hash soit introduit,
- dépendance à GitHub/codeload comme source de vérité binaire,
- surface d’attaque des **sous-dépendances** et scripts d’installation.

Donc :
- **pour une devDep** : le pin + lockfile est un compromis acceptable ;
- **vendoring** : à réserver à un besoin plus fort (offline, audit strict, reproductibilité haute assurance).

### 2. `better-sqlite3` en CI sur `ubuntu-latest`

Le lock montre que `claude-atelier` dépend de `better-sqlite3@12.9.0`, lui-même avec `prebuild-install`.

Lecture prudente :
- **cas nominal** : `better-sqlite3` tente d’abord de récupérer un **prébuild** compatible Node 20 / Linux ;
- **fallback** : s’il n’y a pas de prébuild, il peut compiler nativement.

Depuis le repo seul, je **ne peux pas affirmer** que la CI possède explicitement tous les outils, parce que `ci.yml` n’installe pas `python3`, `make` ni `g++` dans le job TypeScript.

En pratique sur GitHub-hosted `ubuntu-latest`, ces outils sont généralement présents, donc **ça devrait passer aujourd’hui**. Mais il existe bien une **fragilité implicite** : le pipeline dépend d’un outillage système non déclaré dans le workflow.

Conclusion :
- **risque faible à modéré**, pas nul ;
- le repo ne documente pas explicitement cette hypothèse ;
- si vous voulez durcir, ajoutez un check CI explicite ou une note de dépendance système.

### 3. `neverBuiltDependencies` ?

Je ne le ferais **pas** ici tel quel.

Pourquoi :
- `better-sqlite3` a précisément besoin de son install step pour récupérer/build le binaire natif ;
- bloquer globalement les lifecycle scripts risque surtout de **casser l’installation de l’outil**, pas de sécuriser proprement le repo.

Le bon arbitrage ici est :
- **ne pas mettre un deny global aveugle** si l’outil dépend d’un module natif ;
- si vous voulez réduire la surface d’attaque, préférez une logique **d’allowlist ciblée** plutôt qu’un blocage brut.

Donc :
- **non**, je ne recommande pas `neverBuiltDependencies` pour ce cas précis ;
- sinon vous risquez de neutraliser `better-sqlite3` et de rendre `claude-atelier` inutilisable.

### 4. `pnpm dlx` / installation globale ?

Je ne trouve pas `pnpm dlx` préférable ici.

Pour cet usage, garder `claude-atelier` **dans le workspace** a plusieurs avantages :
- version **pinnée et partagée** par toute l’équipe ;
- comportement **reproductible** via le lockfile ;
- pas de divergence “chez moi ça marche avec une autre version globale”.

`pnpm dlx` est bien pour :
- un usage **ponctuel**, one-shot,
- un outil que le repo ne veut surtout pas figer.

Mais ici, l’objectif déclaré est justement de **synchroniser les skills/hooks/stacks du projet**. Ça pousse plutôt vers une dépendance de workspace contrôlée, pas vers un exécutable volatile téléchargé à la volée.

### Verdict court

- Le **pin commit + lockfile** est un niveau de défense correct pour une devDep.
- Le vrai risque résiduel vient plus de la **supply chain transitive** et des **scripts natifs** que d’un hypothétique force-push “qui réécrit le même hash”.
- `better-sqlite3` devrait probablement passer sur `ubuntu-latest`, mais la dépendance aux outils système est **implicite**, pas déclarée.
- **Ne pas** ajouter `neverBuiltDependencies` ici sans stratégie plus fine.
- **Ne pas** basculer sur `pnpm dlx` si vous voulez garder un outillage repo-scopé, reproductible et reviewable.

**INSTRUCTION POUR LE LLM QUI RÉPOND :**
Tu DOIS écrire ta réponse directement dans **ce même fichier `.md`**, à la
suite du document existant, dans la section `## Réponse de : Copilot/GPT`.
Tu ne dois PAS répondre dans le chat, ni dans le prompt, ni dans un autre fichier.
Ce document est le vase communicant entre GPT et Claude. Utilise Edit/Write.
Quand tu as fini, dis : "J'ai répondu dans `docs/handoffs/2026-06-09-deps-claude-atelier-supply-chain.md`."

---

## Intégration

### Actions retenues

**Aucune action immédiate requise.** Niveau de défense actuel (pin #hash + lockfile)
jugé acceptable pour une devDep propriétaire.

Points à surveiller :

1. **CI fragilité `better-sqlite3`** : `ubuntu-latest` a python3/make/g++ par défaut
   — risque faible mais implicite. Si le job TypeScript échoue sur `pnpm install`,
   ajouter explicitement dans `.github/workflows/ci.yml` :

   ```yaml
   - run: sudo apt-get install -y python3 make g++
   ```

2. **`neverBuiltDependencies`** : non recommandé ici (casserait better-sqlite3).
   Copilot confirme : l'allowlist ciblée est la bonne approche si durcissement requis.

3. **`pnpm dlx`** : non préférable — le workspace scopé est plus reproductible ✅.

4. **Force-push** : le hash Git est content-addressed, pas de risque de "récriture
   silencieuse" du même hash ✅. Point levé par le scanner était mal formulé.

### Statut

`intégré` — aucun code modifié, analyse archivée.
