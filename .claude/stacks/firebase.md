---
stack: firebase
applies_to: ["firebase.json", ".firebaserc", "firestore.rules", "storage.rules", "functions/**"]
loads_from: src/fr/CLAUDE.md §0 (Contexte projet)
status: stub
figure: Camille
---

# Stack — Firebase

> Auth Google ? Un switch. Storage ? Un switch. Base de données ? Encore un switch.
> Brancher les câbles, c'est pour ceux qui ont du temps. Camille, elle a pas le temps.
> *— Deploy first, understand later.* — Camille 🔥

## Périmètre prévu (P3)

- **Sécurité des règles Firestore / Storage** : jamais de `allow read, write: if true`
- **Clés côté client** : Firebase apiKey n'est pas un secret mais les règles
  doivent toujours être défensives
- **Cloud Functions** : idempotence, gestion des cold starts, timeouts
- **Firestore** : indexation composite, structure de données denormalisée
  assumée, pagination stricte
- **Auth** : providers, custom claims, revalidation des tokens
- **Emulator Suite** : obligatoire en dev, jamais de tests contre prod
- **Secrets runtime** : Secret Manager pour Functions, jamais `.env` committé
