---
kind: security
name: pre-push-gate
loads_from: src/fr/CLAUDE.md §24
replaces: src/fr/security/_legacy.md (partiel)
---

# Security — Pre-push Gate

> Chargé à la demande. Documente le gate pré-push qui vérifie
> l'intégrité avant tout `git push`. Le script réel est dans
> `../../../scripts/pre-push-gate.sh`.

## Principe

**Aucun `git push` ne doit être exécuté sans que le gate ait passé.**
C'est la dernière ligne de défense avant que du code (potentiellement
avec des secrets ou des régressions) atteigne le remote.

## Les 5 étapes

```text
1. Audit secrets       → patterns dangereux dans le diff staged
2. Fichiers sensibles  → .env, .pem, .key trackes par git
3. Lint                → linter de la stack (eslint, ruff, checkstyle)
4. Build               → compilation (npm build, mvn compile, etc.)
5. Tests               → suite de tests (npm test, pytest, etc.)
```

Chaque étape a un verdict : `[PASS]` ou `[FAIL]`. Un seul `[FAIL]`
bloque le push avec exit code 1.

## Usage

```bash
# Gate complete (5 checks)
bash scripts/pre-push-gate.sh

# Gate rapide (secrets + fichiers sensibles seulement)
bash scripts/pre-push-gate.sh --quick
```

## Détection automatique de stack

Le script détecte la stack depuis les fichiers projet :

| Fichier détecté | Stack | Lint | Build | Tests |
| --- | --- | --- | --- | --- |
| `package.json` | Node.js | `npm run lint` | `npm run build` | `npm test` |
| `pyproject.toml` / `requirements.txt` | Python | `ruff check .` | — | `pytest` |
| `pom.xml` | Maven | `mvn checkstyle:check` | `mvn compile` | `mvn test` |
| `build.gradle` | Gradle | `checkstyleMain` | `compileJava` | `test` |
| Rien détecté | Inconnu | Skip + warning | Skip + warning | Skip + warning |

## Installation comme hook git

```bash
# Installer une fois par projet
cat > .git/hooks/pre-push << 'EOF'
#!/bin/bash
bash scripts/pre-push-gate.sh
EOF
chmod +x .git/hooks/pre-push
```

## Règle absolue

- **Jamais de `--no-verify`** : contourner le hook annule toute la
  protection. Si le gate bloque, corriger le problème, pas le contourner.
- Claude ne doit **jamais** suggérer `--no-verify` comme solution.
- Si le gate est trop strict pour un cas légitime : modifier le gate
  (pas le contourner).

## Anti-patterns

- `git push --no-verify` pour « gagner du temps »
- Gate qui bloque mais qu'on ignore (perte de confiance dans l'outil)
- Gate qui ne tourne jamais (installée mais jamais exécutée)
- Gate trop large qui flag des faux positifs constants (calibrer les regex)

## Voir aussi

- `./secrets-rules.md` — détail des patterns détectés
- `./emergency.md` — que faire si un secret a quand même fuité
