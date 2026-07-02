# AGENTS.md — Règles communes à tous les agents

> **Hiérarchie des fichiers de configuration :**
> - Ce fichier contient les règles universelles applicables à **tous les agents** (Claude, Copilot, Gemini, Codex…)
> - Les fichiers agent-spécifiques (`CLAUDE.md`, `.github/copilot-instructions.md`, `GEMINI.md`…) ne contiennent que le **delta propre à chaque agent**
> - En cas de conflit entre ce fichier et un fichier agent-spécifique, **AGENTS.md prime** sur les règles communes
>
> Généré par [claude-atelier](https://github.com/malikkaraoui/claude-atelier)

---

## Flow de traitement

**Explore → Plan → Implement → Verify.**

- **Explore** : fichiers concernés uniquement
- **Plan** : impacts + dépendances avant d'écrire
- **Implement** : minimal viable · Edit ciblé toujours — jamais réécriture complète si > 20 lignes non modifiées
- **Verify** : tests + gate pré-push

Mode rapide (< 2 fichiers, non critique) : Implement → Verify seulement.

---

## Anti-hallucination — règle absolue

Interdit d'inventer : faits, commandes, API, options, chiffres, comportements non vus.
Si incertain → « Je ne peux pas l'affirmer » + 2–3 hypothèses étiquetées + comment vérifier.
Info récente ou instable → signaler explicitement.

---

## Gestion des erreurs

Une tentative corrective directe. Échec → changer d'approche, jamais itérer à l'identique. Produire hypothèses + points de rupture + stratégie alternative.

---

## Qualité du code

Prêt prod, pas sur-ingénié : validation d'inputs, erreurs propres, logs utiles, commentaires si non trivial. Plusieurs approches → recommander la plus robuste, 2 lignes de justification max.

---

## Anti-patterns

Refus : duplication, sur-ingénierie, optimisation prématurée, fonctions > 30 lignes sans raison, logique dispersée. Règle : logique réutilisée ≥ 2 fois → extraire.

---

## Architecture

Template par défaut : `/core` · `/modules` · `/services` · `/utils` · `/tests`. Projets opinionnés (Next.js, Django…) → suivre la convention du framework. Détails : `templates/project-structure.md`.

---

## Tests

Obligatoires si logique métier, transformation, comportement critique. Couvrir nominal + edge cases + erreurs. `npm test` (ou équivalent) doit passer avant chaque push.

---

## Code Review

Déclenchement : après feature, audit global, blocage. La règle anti-hallucination prime : jamais de critique inventée pour remplir une section.

---

## Git Workflow

Commits atomiques, messages en français, **jamais signer** (pas de trailer `Co-Authored-By`, `Signed-off-by` ou similaire). Checkpoint avant action risquée. `git push` toujours précédé de la gate pré-push.

---

## Cloud / CI-CD

Stateless, idempotent, secrets externalisés, IaC, fail fast, tests locaux avant déploiement.
