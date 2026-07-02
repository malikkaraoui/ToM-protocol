# Context7 — Mapping Phase × Stack

> Satellite chargé automatiquement via §10 du CLAUDE.md core.
> Objectif : à chaque message, croiser §0 (Phase + Stack) avec ce fichier
> pour savoir quels livres de doc porter — et lesquels laisser de côté.

## Règle d'application

1. Lire §0 → extraire `Phase` et `Stack`
2. Chercher la ligne correspondante dans les tableaux ci-dessous
3. Activer context7 (`query-docs`) **uniquement** sur les libs listées
4. Sur tout le reste : répondre depuis la mémoire sans appel context7

Si §0 est vide (`—`) → **ne pas charger context7**, signaler à Malik : « §0 est vide, indique-moi la phase et la stack pour calibrer les docs. »

---

## Phases — Comportement général

| Phase | context7 | Modèle conseillé | Priorité |
|---|---|---|---|
| Brainstorming / Idéation | ❌ Aucun | Sonnet | Espace de réflexion, pas de docs |
| Architecture / Conception | ⚠️ Architecture seulement | Opus | Patterns, ADR, pas d'API détaillée |
| Documentation | ⚠️ Uniquement la lib documentée | Sonnet | Ciblé sur le sujet écrit |
| Implémentation | ✅ Stack complète | Sonnet | Charger tous les livres de la stack |
| Review / Test | ✅ Stack complète | Sonnet | Idem implémentation |
| Release / Production | ⚠️ CI-CD + infra seulement | Sonnet | Pas besoin de toute la stack |
| Maintenance | ⚠️ Lib ciblée par le bug | Haiku → Sonnet | Un seul livre à la fois |

---

## Stacks — Libs context7 à charger

### Langages

| Stack §0 | Libs context7 à activer | Ne pas charger |
|---|---|---|
| `JavaScript` | — (connu, stable) | tout |
| `TypeScript` | typescript | — |
| `Python` | — (connu, stable) | tout |
| `Java` | — sauf si Spring/Quarkus | framework spécifique |
| `Kotlin` | kotlin, ktor si API | — |
| `Rust` | rust (docs.rs), tokio si async | — |
| `Go` | — (stable, bien connu) | tout |
| `C` | — (stable) | tout |
| `C++` | — sauf si Qt/Boost | framework spécifique |
| `C#` | microsoft/dotnet si .NET 9+ | — |
| `Objective-C` | — | tout |
| `Swift` | swift, swiftui si UI | — |
| `PHP` | — sauf si Laravel/Symfony | framework spécifique |
| `Perl` | — (stable) | tout |
| `SQL` | — (stable, spécifique au SGBD) | tout |
| `R` | — sauf si Shiny/tidymodels | framework spécifique |
| `Fortran` | — (stable) | tout |
| `MATLAB` | — (documentation interne) | tout |
| `Ada` | — (GNAT/SPARK docs internes) | tout |
| `Assembly` | — | tout |
| `Delphi` | — (documentation Embarcadero) | tout |
| `Scratch` | — (scratch.mit.edu) | tout |
| `Visual Basic` | microsoft/dotnet si .NET | — |

### Frameworks web

| Stack §0 | Libs context7 à activer |
|---|---|
| `Next.js` | next.js, react |
| `Next.js + Vercel` | next.js, react, vercel, @vercel/ai |
| `React / Vite` | react, vite |
| `FastAPI` | fastapi, pydantic |
| `Django` | django, djangorestframework |
| `Flask` | flask |
| `Spring Boot` | spring-boot, spring-data |
| `NestJS` | nestjs |
| `Express` | — (stable) |

### Cloud & Infra

| Stack §0 | Libs context7 à activer |
|---|---|
| `Firebase` | firebase-admin, firestore, firebase-auth |
| `GCP` | google-cloud-* selon service utilisé |
| `AWS` | aws-sdk selon service (s3, lambda, rds…) |
| `Azure` | azure-sdk selon service |
| `Docker` | — (stable) |
| `Vercel` | vercel, @vercel/ai, next.js |

### IA & LLM

| Stack §0 | Libs context7 à activer |
|---|---|
| `AI SDK` | @vercel/ai |
| `Anthropic SDK` | @anthropic-ai/sdk |
| `LangChain` | langchain |
| `Ollama` | ollama |
| `OpenAI SDK` | openai |

### Spéciaux (projet)

| Stack §0 | Libs context7 à activer |
|---|---|
| `Freebox` | — (API interne, pas dans context7) |
| `ios-xcode` | swift, swiftui, xcode |

---

## Combinaisons fréquentes (raccourcis)

| Nom raccourci | Équivalent Stack §0 | Libs chargées |
|---|---|---|
| `backend-python` | Python + FastAPI + Firebase | fastapi, pydantic, firebase-admin |
| `fullstack-vercel` | Next.js + Vercel + AI SDK | next.js, react, vercel, @vercel/ai |
| `mobile-ios` | Swift + SwiftUI | swift, swiftui |
| `java-spring` | Java + Spring Boot | spring-boot, spring-data |
| `rust-api` | Rust + Tokio | rust, tokio |

---

## Mise à jour §0 — Protocole

Quand Malik change de projet ou de phase, mettre à jour §0 **immédiatement** avec :

```
| Projet courant | NomDuProjet         |
| Phase          | Implémentation      |
| Stack          | FastAPI + Firebase  |
```

→ Claude relit ce fichier, charge les bons livres, ajuste le modèle si nécessaire.

**Signal d'alerte** : si pendant une session le sujet dérive vers une nouvelle stack non listée dans §0, proposer immédiatement : « Stack actuelle dans §0 : X — on migre vers Y ? Je mets à jour. »

---

## Ce que ce fichier ne fait PAS

- Il ne lance pas context7 automatiquement sur tout — uniquement sur appel ciblé
- Il ne remplace pas le jugement : si le LLM sait déjà, pas besoin d'appel
- Il ne couvre pas les libs ultra-stables (git, bash, SQL de base) — jamais utile
