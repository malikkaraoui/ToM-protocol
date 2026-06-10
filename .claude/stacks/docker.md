---
stack: docker
applies_to: ["Dockerfile", "docker-compose.*", ".dockerignore"]
loads_from: src/fr/CLAUDE.md §0 (Contexte projet)
status: stub
figure: Pascal
---

# Stack — Docker

> Sur les docks, on soulève pas des idées. On soulève des volumes.
> Son boulot ? Que ça tourne partout. Ton boulot ? Pas le contredire.
> *— Ship it. Pas d'excuses.* — Pascal 🐳

## Périmètre prévu (P3)

- **Multi-stage builds** par défaut pour réduire la taille finale
- **Utilisateur non-root** dans le final stage (`USER app`)
- **`.dockerignore`** obligatoire, aligné avec `.gitignore`
- **Layer cache** optimisé : `COPY package*.json` avant `COPY .`
- **Healthchecks** pour tout service long-running
- **Secrets runtime** : BuildKit secrets ou vars d'env, jamais `ARG` pour
  des tokens
- **Tags immuables** : `:sha-<git>` plutôt que `:latest` en prod
- **Compose** : `depends_on` + healthchecks, pas de `sleep` en entrypoint
