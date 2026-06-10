# NPM Publish — CI/CD automatique

> Plus de `npm publish` manuel. Push un tag, npm reçoit le paquet.

## Le problème

Publier manuellement c'est :
- Oublier de lancer les tests avant
- Se battre avec le 2FA à chaque fois
- Oublier de bumper la version
- Publier avec un README pas à jour

## La solution

Un workflow GitHub Actions qui publie automatiquement sur npm à chaque
push d'un tag `v*`.

```text
npm version patch/minor/major
    ↓
git push && git push --tags
    ↓
GitHub Actions déclenché sur tag v*
    ↓
Lint + vérification version tag = package.json
    ↓
npm publish --access public (via NPM_TOKEN)
```

## Setup (une seule fois)

### 1. Générer un token npm

1. Aller sur [npmjs.com](https://www.npmjs.com) → Account → Access Tokens
2. Créer un token **Classic** de type **Automation**
3. Copier le token (commence par `npm_...`)

**Important :** le token doit être de type **Automation** (pas Granular).
Les tokens Granular ne permettent pas le publish depuis la CI.

### 2. Ajouter le secret GitHub

1. Repo GitHub → Settings → Secrets and variables → Actions
2. New repository secret
3. Nom : `NPM_TOKEN`
4. Valeur : le token npm copié

### 3. Le workflow

Le fichier `.github/workflows/npm-publish.yml` est inclus dans l'atelier.
Il se déclenche automatiquement sur tout push de tag `v*`.

## Utilisation quotidienne

```bash
# Patch (0.2.2 → 0.2.3)
npm version patch && git push && git push --tags

# Minor (0.2.3 → 0.3.0)
npm version minor && git push && git push --tags

# Major (0.3.0 → 1.0.0)
npm version major && git push && git push --tags
```

`npm version` fait trois choses :
1. Bumpe la version dans `package.json`
2. Crée un commit `v0.x.x`
3. Crée le tag git `v0.x.x`

Ensuite `git push --tags` déclenche le workflow → publication automatique.

## Vérifications intégrées

Le workflow vérifie avant de publier :
- **Lint** : `npm run lint` passe
- **Version match** : le tag git correspond à la version dans `package.json`
- Si l'un échoue → pas de publication, erreur visible dans l'onglet Actions

## Troubleshooting

| Problème | Cause | Fix |
| --- | --- | --- |
| `EOTP` / 2FA requis | Token de type Granular | Recréer en type **Automation** |
| 403 Forbidden | Token expiré ou mauvais scope | Recréer le token |
| Version mismatch | Tag et package.json désynchronisés | `npm version patch` crée les deux ensemble |
| Workflow ne se déclenche pas | Tag pas pushé | `git push --tags` |

## Détection automatique des mises à jour

L'atelier détecte automatiquement quand une nouvelle version est disponible :

- **Hook `routing-check.sh`** : vérifie toutes les 30 min via le diagnostic
  throttlé → affiche `🆕 claude-atelier 0.2.2 → 0.3.0 disponible`
- **CLI** : chaque commande (`init`, `doctor`, `lint`) affiche un avertissement
  en fin d'exécution si une version plus récente existe sur npm
- **`bin/init.js`** : recommandation post-install si la version locale est obsolète

L'utilisateur n'a jamais besoin de vérifier manuellement.
