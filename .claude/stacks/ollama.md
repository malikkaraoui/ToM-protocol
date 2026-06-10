---
stack: ollama
applies_to: ["Modelfile", "**/ollama*"]
loads_from: src/fr/CLAUDE.md §0 (Contexte projet)
status: stub
figure: Jeffrey
---

# Stack — Ollama

> Pas d'API key. Pas de facture. Pas de dependency.
> Ton modèle tourne chez toi, tes données restent chez toi.
> *— Local first. Always.* — Jeffrey 🦙

## Périmètre prévu (P3)

- **Modèle local vs cloud** : quand basculer sur Ollama plutôt qu'un
  provider cloud (latence, confidentialité, coût)
- **Modelfile** : convention de nommage, FROM, PARAMETER, SYSTEM
- **Gestion mémoire** : VRAM requirements par taille de modèle, fallback
  CPU si insuffisant
- **API locale** : endpoint `http://localhost:11434`, format OpenAI-compat,
  streaming
- **Quantization** : choisir Q4_K_M / Q5_K_M / Q8 selon VRAM/qualité
- **Embeddings** : modèles dédiés (nomic-embed-text, mxbai-embed-large)
- **Sécurité** : jamais exposer l'API Ollama publiquement sans proxy auth
- **Benchmark** : tokens/s + qualité avant de committer sur un modèle
