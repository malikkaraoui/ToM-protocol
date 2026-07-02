---
name: la-bise
description: "Organise les échanges inter-LLM : prépare le contexte pour GPT/Mistral et intègre leurs réponses. Vent léger entre Claude et ses pairs — ni une fusion, ni un silence."
figure: La Bise
---

# La Bise

> 🌬️ Entre deux modèles, pas d'embrassade — juste une bise.
> Un souffle de contexte dans la bonne direction.
> Mistral fait rage sur la toile. La Bise, elle, passe là où il le faut.

Coordonne les échanges de contexte entre Claude et d'autres LLMs (GPT-4o, Mistral, Ollama local).

## Quand utiliser

- Second avis GPT sur une décision d'architecture ou un choix technique
- Déléguer une sous-tâche à Mistral/Ollama local (économie de tokens)
- Recevoir un output GPT/Mistral et l'intégrer proprement dans la session

## Procédure

### Étape 1 — Identifier la cible et le sujet

"Vers quel modèle envoyer ?
1. GPT-4o (ChatGPT) — second avis, raisonnement
2. Mistral (Ollama local) — tâche légère, économie tokens
3. Autre LLM externe"

"Quel est l'objet de l'échange ?
1. Second avis / validation
2. Délégation d'une sous-tâche
3. Intégration d'une réponse déjà reçue"

### Étape 2 — Préparer le brief

Générer un brief compact (≤ 500 tokens) :

```
# Contexte — [projet]
[2-3 lignes : stack, phase, contraintes clés]

# Sujet
[question précise ou tâche à déléguer]

# État actuel
[ce qui existe, fichiers clés, approche en cours]

# Ce qu'on attend
[format de réponse, périmètre strict]

---
RÈGLES :
- Ne pas modifier de code source
- Rester dans le périmètre défini
- Répondre de façon concise et actionnable
```

### Étape 3 — Afficher le prompt copier-coller

Afficher à l'utilisateur le brief prêt à coller dans l'interface cible (ChatGPT, Mistral.ai, ollama run mistral).

### Étape 4 — Intégrer la réponse

Quand l'utilisateur revient avec la réponse reçue :
1. Synthétiser les points actionnables
2. Trier : appliquer maintenant / garder en mémoire / ignorer
3. Proposer les actions concrètes

## Règles

- La Bise ne remplace pas Mohamed (review formelle) — elle complète pour les échanges ad hoc
- Brief ≤ 500 tokens — sinon le contexte est trop lourd pour passer proprement
- Toujours préciser le périmètre : le LLM cible ne connaît pas l'historique de session
- Pas de secrets, clés, tokens dans le brief
