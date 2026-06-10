---
name: audit-safe
description: "Audit sécurité complet : secrets scan, pre-push gate, permissions, .claudeignore. Utiliser avant un push, avant de dormir, ou quand un doute sur la sécurité."
figure: Inspecteur
---

# Audit Safe

> L'Inspecteur ouvre les tiroirs un par un. Secrets, permissions, gate,
> fichiers sensibles. Rien ne sort de l'atelier sans son visa.

Audit sécurité complet. Vérifie tout ce qui pourrait fuiter ou casser.

## Procédure

### Étape 1 — Pre-push gate

Lance `bash scripts/pre-push-gate.sh --quick` (secrets + fichiers sensibles).
Si le script n'existe pas → signaler et proposer `/atelier-setup`.

### Étape 2 — Secrets dans le diff

```bash
git diff --cached | grep -iE \
  "(api_key|secret|token|password|private_key|BEGIN RSA|sk-|AIza|AKIA|ghp_)"
```

Si match → **BLOQUER**, lister les lignes, ne pas continuer.

### Étape 3 — Fichiers sensibles trackés

```bash
git ls-files | grep -E "\.(env|pem|key|p12|pfx|keystore)$"
```

Si match → lister et recommander `git rm --cached`.

### Étape 4 — Vérification .claudeignore

Vérifier que `.claudeignore` contient au minimum :
`.env`, `*.pem`, `*.key`, `*credentials*.json`, `*secret*.json`.

Si incomplet → afficher ce qui manque.

### Étape 5 — Vérification permissions settings.json

Lire `.claude/settings.json` et vérifier la deny list :
- `git push --force` doit être en deny
- `sudo` doit être en deny
- `rm -rf` doit être en deny

### Étape 6 — Rapport

```text
╔══════════════════════════════════════════════════╗
║  🔒 Audit Safe — Rapport                        ║
╠══════════════════════════════════════════════════╣
║  [✅/❌] Pre-push gate                           ║
║  [✅/❌] Secrets dans le diff                    ║
║  [✅/❌] Fichiers sensibles trackés              ║
║  [✅/❌] .claudeignore complet                   ║
║  [✅/❌] Deny list settings.json                 ║
╠══════════════════════════════════════════════════╣
║  Verdict : [SAFE / X PROBLÈMES]                ║
╚══════════════════════════════════════════════════╝
```

## Règles

- Ne modifie aucun fichier automatiquement (signaler, pas corriger)
- Si un secret est détecté → STOP, procédure d'urgence (voir `security/emergency.md`)
- Recommander les corrections, ne pas les appliquer sans validation
