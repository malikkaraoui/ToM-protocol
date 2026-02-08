API Reference (OpenAPI)

Pourquoi OpenAPI ici

GitBook peut transformer un fichier OpenAPI (YAML/JSON) en blocs interactifs et testables (“Test it”, propulsé par Scalar).
Dans ToM aujourd’hui, la majorité du trafic applicatif est WebRTC (DataChannels) et le signaling est WebSocket — donc OpenAPI n’est pas la meilleure grammaire pour documenter le protocole P2P.

Ce qu’on fait quand même

On fournit un petit OpenAPI pour:
- documenter un endpoint HTTP de santé (readiness) du signaling server
- montrer comment utiliser les extensions GitBook (x-page-title, x-parent, x-codeSamples, x-stability, x-internal…)
- préparer une future “control plane” HTTP si besoin (observabilité, introspection, debug)

Fichiers

- openapi/tom-signaling.openapi.yaml

Intégration dans GitBook

Option A — importer le fichier
- Ajouter une page “API Reference”
- Insérer un bloc OpenAPI
- Sélectionner le fichier YAML (upload) ou fournir une URL si vous l’hébergez

Option B — publication via CI
GitBook propose une CLI qui peut publier/mettre à jour une spec via token:
- `npx -y @gitbook/cli@latest openapi publish ...`

Bonnes pratiques

- Utiliser `tags` + `x-parent` pour structurer le TOC automatiquement.
- Mettre des `x-codeSamples` alignés avec notre SDK (quand on a un vrai client HTTP).
- Marquer ce qui est expérimental via `x-stability: experimental`.

Référence

- Extensions supportées par GitBook: https://gitbook.com/docs/api-references/extensions-reference
