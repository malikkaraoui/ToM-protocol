---
kind: stack
name: npm-publish
loads_from: src/fr/CLAUDE.md §10
triggers: npm publish, npm version, package.json version, registry, npmjs, npmrc, npm token, npm tag
---

# Stack — NPM Publish

> **Agent : Isaac** 📦
>
> L'atelier prépare une livraison. Isaac entre en scène — il connaît
> le registre, les tokens, les tags, le versionning sémantique. Il publie
> proprement, vérifie avant d'envoyer, et ne laisse jamais un paquet
> partir sans que les tests soient passés.
>
> *« npm install — deux mots qui doivent toujours marcher. »*

## Philosophie

Publier sur npm, c'est livrer du code à des inconnus. Pas de place
pour l'à-peu-près :
- **Jamais de publish manuel** — GitHub Actions s'en charge
- **Jamais sans tests** — le workflow lint avant de publier
- **Jamais sans version cohérente** — tag git = package.json, vérifié automatiquement
- **Jamais sans token Automation** — les tokens Granular bloquent en CI

## Pipeline de publication

```text
npm version patch/minor/major
    ↓  crée commit + tag git automatiquement
git push && git push --tags
    ↓  tag v* déclenche GitHub Actions
Lint + version check
    ↓  si tout passe
npm publish --access public (via NPM_TOKEN)
    ↓
registre npmjs.org mis à jour
```

## Versionning sémantique

| Commande | Avant → Après | Quand |
| --- | --- | --- |
| `npm version patch` | 0.2.2 → 0.2.3 | Bug fix, doc, polish |
| `npm version minor` | 0.2.3 → 0.3.0 | Nouvelle feature |
| `npm version major` | 0.3.0 → 1.0.0 | Breaking change |

`npm version` fait trois choses d'un coup :
1. Bumpe `version` dans `package.json`
2. Crée un commit `v0.x.x`
3. Crée le tag git `v0.x.x`

## Commandes courantes

```bash
# Publier une nouvelle version (tout automatisé)
npm version patch && git push && git push --tags

# Vérifier la version publiée
npm view <paquet> version

# Vérifier ce qui sera inclus dans le paquet
npm pack --dry-run

# Voir les fichiers publiés
npm publish --dry-run

# Voir les versions publiées
npm view <paquet> versions --json

# Déprécier une version
npm deprecate <paquet>@<version> "raison"
```

## Setup CI/CD (une seule fois)

### 1. Token npm
- npmjs.com → Account → Access Tokens
- Type : **Classic → Automation** (pas Granular)
- Copier le token `npm_...`

### 2. Secret GitHub
- Repo → Settings → Secrets → Actions
- Nom : `NPM_TOKEN`
- Valeur : le token

### 3. Workflow
Le fichier `.github/workflows/npm-publish.yml` se déclenche sur tout
push de tag `v*`. Il lint, vérifie la cohérence tag/version, et publie.

## package.json — Champs essentiels

```json
{
  "name": "mon-paquet",
  "version": "1.0.0",
  "description": "Description courte et claire",
  "main": "index.js",
  "bin": { "mon-cli": "bin/cli.js" },
  "files": ["bin/", "src/", "README.md", "LICENSE"],
  "publishConfig": { "access": "public" },
  "engines": { "node": ">=18.0.0" },
  "repository": { "type": "git", "url": "..." },
  "license": "MIT"
}
```

Champs critiques :
- **`files`** : ce qui est inclus dans le paquet (whitelist)
- **`publishConfig.access`** : `public` pour les paquets scoped
- **`engines`** : version Node minimum supportée

## Troubleshooting

| Problème | Cause | Fix |
| --- | --- | --- |
| `EOTP` / 2FA requis | Token de type Granular | Recréer en type **Automation** |
| 403 Forbidden | Token expiré ou mauvais scope | Recréer le token |
| Version mismatch en CI | Tag et package.json désynchronisés | `npm version patch` crée les deux ensemble |
| Workflow ne se déclenche pas | Tag pas pushé | `git push --tags` |
| `npm ERR! 402` | Paquet scoped sans `--access public` | Ajouter `publishConfig.access: "public"` |
| Fichiers manquants dans le paquet | `files` trop restrictif | `npm pack --dry-run` pour vérifier |
| `EPERM` / Permission denied | Mauvais registre | Vérifier `.npmrc` |

## Bonnes pratiques

- **Toujours `npm pack --dry-run`** avant un premier publish pour vérifier le contenu
- **CHANGELOG.md** : mettre à jour avant chaque version
- **`npm version`** plutôt que modifier `package.json` à la main
- **Ne jamais publier de secrets** : vérifier que `.npmignore` ou `files` exclut `.env`, credentials, etc.
- **README visible** : c'est la vitrine sur npmjs.com — soigner le contenu
