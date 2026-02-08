MCP (docs publiées GitBook)

Résumé

GitBook génère automatiquement un serveur MCP pour chaque site de documentation publié.
Ce MCP sert à donner à un assistant IA un accès en lecture seule au contenu publié (jamais aux brouillons).

C’est différent d’un “MCP GitBook” qui utiliserait l’API GitBook (et donc un token) pour créer/modifier des pages.

URL du serveur MCP

Prends l’URL de ton site GitBook publié et ajoute :

/~gitbook/mcp

Exemples :
- Site docs : https://gitbook.com/docs
- MCP : https://gitbook.com/docs/~gitbook/mcp

Important : ouvrir cette URL dans un navigateur peut afficher une erreur. Elle est destinée aux outils qui font des requêtes HTTP MCP.

Pré-requis

- Le site GitBook doit être publié.
- Le serveur MCP n’expose que le contenu publié (read-only).
- Il respecte la visibilité :
  - site public ⇒ MCP public
  - site authentifié ⇒ MCP authentifié
- Transport : HTTP uniquement (pas stdio, pas SSE).

Pourquoi c’est “mieux” (souvent)

Pour un assistant IA (VS Code extension, Cursor, Claude Desktop…), c’est généralement la meilleure voie :
- pas besoin de token API GitBook si le site est public
- contenu toujours à jour sur la dernière version publiée
- très simple à configurer (une URL)

Ce que ça ne fait pas

- Ça ne crée pas un GitBook à partir de ton repo.
- Ça ne pousse pas des pages automatiquement.
- Ça ne remplace pas Git Sync / import / API d’édition.

“Faire un GitBook” pour ToM : les options

1) Git Sync (recommandé)
- Connecter un space à ce repo et synchroniser le dossier docs/gitbook/
- L’édition se fait via PRs (docs-as-code)
- Une fois publié, tu utilises ensuite le MCP du site pour donner le contexte aux assistants

2) Import manuel
- Créer un space/site, copier le contenu des fichiers docs/gitbook/
- Publier le site
- Utiliser ensuite /~gitbook/mcp

3) API/CLI GitBook (token requis)
- Pour publier des specs OpenAPI via CI, GitBook propose une CLI (openapi publish)
- Pour éditer GitBook par API, il faut un vrai token API GitBook et une config serveur correcte

Option “Connect with MCP server”

Dans les réglages de Customization > Page actions, tu peux activer un bouton/menu “Connect with MCP server” pour copier facilement l’URL MCP depuis le site publié.

Références

- MCP servers for published docs : https://gitbook.com/docs/publishing-documentation/mcp-servers-for-published-docs
- Customization / Page actions : https://gitbook.com/docs/publishing-documentation/customization/extra-configuration#page-actions
