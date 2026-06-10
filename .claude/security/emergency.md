---
kind: security
name: emergency
loads_from: src/fr/CLAUDE.md §22
replaces: src/fr/security/_legacy.md (partiel)
---

# Security — Procédure d'urgence : secret committé

> Chargé à la demande. Procédure à suivre **immédiatement** si un secret
> (clé API, token, credentials) a été committé dans le repo.
>
> Le temps est critique : plus un secret reste dans l'historique git,
> plus il est exposé (bots qui scannent GitHub, caches, forks).

## Les 4 étapes — dans cet ordre exact

### 1. RÉVOQUER la clé immédiatement

**Avant de toucher au git.** La priorité n'est pas de nettoyer le repo,
c'est de rendre la clé inutilisable.

| Service | Où révoquer |
| --- | --- |
| Anthropic / OpenAI | Console développeur → API Keys |
| Google Cloud / Firebase | Console GCP → IAM → Service accounts / API keys |
| AWS | Console IAM → Access keys |
| GitHub | Settings → Developer settings → Personal access tokens |
| Autre | Consulter la doc du service immédiatement |

### 2. SUPPRIMER de l'historique git

```bash
# git filter-branch (compatible partout)
git filter-branch --force --index-filter \
  "git rm --cached --ignore-unmatch CHEMIN_DU_FICHIER" \
  --prune-empty --tag-name-filter cat -- --all

# OU git filter-repo (plus moderne, plus rapide)
git filter-repo --invert-paths --path CHEMIN_DU_FICHIER
```

> ⚠️ `filter-branch` et `filter-repo` réécrivent l'historique. Tous les
> SHAs changent. Les collaborateurs devront re-cloner.

### 3. FORCE PUSH

```bash
git push origin --force --all
git push origin --force --tags
```

> C'est le **seul cas autorisé** pour un force push dans le workflow
> claude-atelier. Voir §22 et `../../templates/settings.json`
> (`git push --force` est en `deny` par défaut — le contourner ici
> nécessite une action manuelle explicite et délibérée).

### 4. NOTIFIER les collaborateurs

- Prévenir tous les collaborateurs de **re-cloner** le repo
  (leur copie locale a les anciens SHAs avec le secret)
- Si le repo est public : considérer que le secret a été compromis
  **même si** l'exposition a été très courte (bots scannent en
  continu)
- Si le repo a des forks : les forks conservent le secret dans leur
  historique. Contacter les mainteneurs des forks.

## Timeline de réponse

| Urgence | Délai maximum |
| --- | --- |
| Révoquer la clé | **< 5 minutes** après détection |
| Nettoyer l'historique | **< 1 heure** |
| Notifier les collaborateurs | **< 2 heures** |
| Re-créer une nouvelle clé | Après revue de l'incident |

## Post-mortem

Après l'urgence, documenter :

1. **Quelle clé a fuité** (type, service, scope de permissions)
2. **Comment elle s'est retrouvée dans le code** (erreur humaine ? CI ?
   copier-coller ?)
3. **Quelle barrière a manqué** (`.gitignore` absent ? gate non installée ?
   `--no-verify` utilisé ?)
4. **Action corrective** pour que ça ne se reproduise pas

## Anti-patterns

- Nettoyer le repo AVANT de révoquer la clé (le secret est exploitable
  tant qu'il est valide, peu importe qu'il soit dans git ou non)
- `git commit -m "remove secret"` sans `filter-branch` : le secret
  reste dans l'historique des commits précédents
- Paniquer et `rm -rf .git` : perte de tout l'historique
- Ne pas notifier les collaborateurs : ils ont encore le secret en local

## Voir aussi

- `./secrets-rules.md` — comment éviter d'en arriver là
- `./pre-push-gate.md` — le gate qui devrait catcher avant push
