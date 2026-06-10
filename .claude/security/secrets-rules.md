---
kind: security
name: secrets-rules
loads_from: src/fr/CLAUDE.md §22
replaces: src/fr/security/_legacy.md (partiel)
---

# Security — Règles sur les secrets

> Chargé à la demande. Règles absolues concernant les secrets, clés,
> tokens et fichiers sensibles. Non négociable — §22 dans la hiérarchie §21.

## Principe absolu

Claude ne commit jamais un fichier contenant un secret.
Claude ne push jamais sans gate pré-push.
Pattern suspect détecté → **bloquer, signaler, ne pas continuer.**

## Fichiers interdits de commit

Ces patterns doivent être dans `.gitignore` **et** `.claudeignore`
avant le premier commit de tout projet :

```gitignore
# Env / Secrets
.env
.env.*
*.pem
*.key
*.p12
*.pfx
*.cer
id_rsa
id_rsa.*
id_ed25519
id_ed25519.*
*.keystore

# Service accounts / credentials
google-services.json
GoogleService-Info.plist
serviceAccountKey.json
*credentials*.json
*secret*.json
*token*.json
*-service-account.json

# Cloud provider
.aws/credentials
.aws/config

# Runtime / logs / DB
*.log
*.sqlite
*.db
node_modules/
dist/
.DS_Store
```

## Patterns suspects — refus immédiat si détectés dans un diff

```text
sk-[a-zA-Z0-9]{32,}           → clé Anthropic / OpenAI
AIza[a-zA-Z0-9_-]{35}         → clé Google API
AKIA[A-Z0-9]{16}              → clé AWS Access Key
ya29\.[a-zA-Z0-9_-]+          → token OAuth Google
ghp_[a-zA-Z0-9]{36}           → token GitHub Personal Access
firebase.*apiKey.*"[^"]+"     → clé Firebase dans le code
```

Si un de ces patterns apparaît dans un `git diff --cached` ou un fichier
ouvert : **stopper immédiatement**, signaler à l'utilisateur, ne pas
continuer le workflow.

## Audit manuel avant push

```bash
# Secrets dans le diff staged
git diff --cached | grep -iE \
  "(api_key|secret|token|password|private_key|BEGIN RSA|sk-|AIza|AKIA|ghp_)"

# Fichiers sensibles non ignorés
git status --short | grep -E "\.(env|pem|key|json)" | grep "^\?\?"

# Fichiers sensibles déjà trackés
git ls-files | grep -E "\.(env|pem|key)"
```

## Bootstrap de sécurité pour un nouveau projet

Voir templates : `../../templates/.gitignore` et `../../templates/.claudeignore`.

À exécuter **avant tout premier commit** :

1. Copier `.gitignore` depuis le template
2. Copier `.claudeignore` depuis le template
3. Vérifier que les deux sont cohérents (`.claudeignore` ⊆ `.gitignore`)
4. `git add .gitignore .claudeignore && git commit -m "chore: ajouter gitignore et claudeignore"`

## Voir aussi

- `./pre-push-gate.md` — le gate automatisé qui vérifie avant push
- `./emergency.md` — procédure si un secret a déjà été committé
