---
name: review-copilot
description: "Génère un handoff review structuré pour Copilot/GPT. Utiliser après une feature, un bug fix, ou quand > 100 lignes ont changé. Aussi déclenché automatiquement par §25."
figure: Mohamed
---

# Review Copilot

> Mohamed 📋 passe en arrière-salle, relit les commits un par un,
> compte les lignes, formule la question précise — et dépose le dossier
> sur le bureau de Copilot sans un mot de trop.
>
> *"Un code non challengé n'est pas fini. C'est une bombe à retardement."*

Handoff structuré pour Copilot/GPT, créé dans `docs/handoffs/`.

## Procédure

### Étape 1 — Collecter le contexte

Exécute silencieusement :

```bash
# Stats depuis le dernier handoff (ou les 20 derniers commits)
git log --oneline -20
git diff --stat HEAD~10 2>/dev/null || git diff --stat HEAD~5
ls -lt docs/handoffs/*.md 2>/dev/null | head -1
```

### Étape 2 — Demander le sujet

"Quel est le sujet de cette review ?
1. Review générale (tout ce qui a changé récemment)
2. Feature spécifique : [laquelle ?]
3. Bug fix : [lequel ?]
4. Architecture / décision technique"

### Étape 3 — Générer le handoff

Créer le fichier `docs/handoffs/YYYY-MM-DD-<sujet-slug>.md` avec
le format suivant :

```markdown
# Handoff — [sujet]

> Date : [date]
> Type : review
> Priorité : [haute si bug fix critique, moyenne sinon]
> reviewedRange: [sha-from]..[sha-to]

---

## De : Claude (Opus 4.6)

### Contexte
[résumé de ce qui a été fait, basé sur les commits récents]

### Question précise
[formulée selon le choix de l'utilisateur à l'étape 2]

### Fichiers à lire
[les fichiers les plus modifiés, max 10]

### Contraintes / hors scope
- Ne pas proposer de réécrire ce qui fonctionne
- Se concentrer sur ce qui manque ou est fragile

---

## Réponse de : Copilot/GPT

**INSTRUCTION POUR LE LLM QUI REPOND :**
Tu DOIS écrire ta réponse directement dans **ce même fichier `.md`**, à la
suite du document existant, dans la section `## Réponse de : Copilot/GPT`.
Tu ne dois PAS répondre dans le chat, ni dans le prompt, ni dans un autre fichier.
Ce document est le vase communicant entre GPT et Claude. Utilise Edit/Write.
Quand tu as fini, dis : "J'ai répondu dans [chemin du fichier]."

---

## Intégration
<!-- Claude remplit après lecture de la réponse -->
```

### Étape 4 — Générer le prompt copier-coller

Afficher à l'utilisateur :

"Handoff créé : `docs/handoffs/[fichier].md`

**Copie ce prompt dans Copilot ↓**

---
[Contenu complet de la section 'De : Claude' du handoff]
---

Quand Copilot répond (dans le fichier ou dans le chat), dis-moi
et je remplirai la section Intégration."

### Étape 5 — Committer le handoff

```bash
git add docs/handoffs/[fichier].md
git commit -m "docs: handoff review [sujet]"
```

## Règles

- Un handoff par review (pas de méga-fichier)
- Le prompt copier-coller doit être **complet et autonome**
- Inclure les fichiers à lire (Copilot en a besoin pour le contexte)
- Committer le handoff (traçabilité)
