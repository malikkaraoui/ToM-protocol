GitBook pack (ToM Protocol)

Objectif

Ce dossier contient une version "GitBook-ready" des contenus clés du repo ToM Protocol.
L’idée est de pouvoir importer ce dossier dans GitBook (ou de le synchroniser via Git) sans devoir retoucher tous les fichiers existants du repo.

Contenu

- SUMMARY.md : table des matières (structure de la doc)
- home.md : page d’accueil
- getting-started.md : installation + quickstart
- concepts.md : concepts de base (rôles, relays, ACK, TTL)
- mcp-published-docs.md : connecter un assistant IA au MCP d’un site GitBook publié
- signaling-server.md : protocole de signaling (WebSocket)
- architecture.md : architecture + ADRs (résumé + liens vers sources)
- design-decisions.md : les 7 décisions verrouillées
- api-reference.md : comment intégrer une spec OpenAPI dans GitBook
- openapi/tom-signaling.openapi.yaml : spec OpenAPI (endpoint HTTP /health du signaling server)
- contributing.md : modèle micro-session
- issues-backlog.md : backlog d’issues prêtes à copier
- whitepaper-v1.md : lien + résumé
- whitepaper-v2.md : plan (à compléter)
- changelog.md : lien + notes

Sources

Ces pages sont dérivées des fichiers du repo :
- README.md
- llms.txt
- CLAUDE.md
- _bmad-output/planning-artifacts/prd.md
- _bmad-output/planning-artifacts/architecture.md
- _bmad-output/planning-artifacts/design-decisions.md
- _bmad-output/implementation-artifacts/epic-4-8-retro-2026-02-07.md
- .github/ISSUE_BACKLOG.md
- CHANGELOG.md
- tom-whitepaper-v1.md

GitBook MCP

Le workspace contient des outils MCP GitBook, mais l’accès nécessite un token valide.
Un placeholder a été ajouté dans .env.example à la racine du repo.

Important

- Ne commitez jamais un token : `.env` est ignoré par git.
- Selon votre configuration, le serveur MCP GitBook ne lit pas automatiquement `.env`.
	Il faut souvent déclarer le token dans la configuration du serveur MCP (env vars) et redémarrer le serveur.

OpenAPI dans GitBook

GitBook peut générer une API Reference interactive depuis une spec OpenAPI (YAML/JSON) et supporte des extensions `x-...`.
Références utiles:
- Extensions OpenAPI GitBook: https://gitbook.com/docs/api-references/extensions-reference
- Structurer une API reference (tags + x-parent): https://gitbook.com/docs/api-references/guides/structuring-your-api-reference
- Ajouter des `x-codeSamples`: https://gitbook.com/docs/api-references/guides/adding-custom-code-samples
- Publication via CI/CD (CLI GitBook): https://gitbook.com/docs/api-references/guides/support-for-ci-cd-with-api-blocks
