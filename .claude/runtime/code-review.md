---
kind: runtime
name: code-review
loads_from: src/fr/CLAUDE.md §12
---

# Runtime — Code Review

> Chargé à la demande. Définit le format et la discipline d'une code review
> produite par Claude (sur un diff, une PR, un fichier).
>
> Source historique : §12 de `CLAUDE-core.md` (P1), corrigé en P2.1.

## Déclenchement

- Après une feature terminée
- Audit global demandé explicitement
- Blocage persistant qui nécessite du recul

## Template de sortie

```markdown
# Code Review

## Problèmes identifiés
<!-- Vides autorisés si aucun problème réel n'est détecté. -->
<!-- Ne jamais inventer. §5 prime. -->

## Angles morts détectés
<!-- Vides autorisés si aucun angle mort réel n'est détecté. -->

## Actions correctives
<!-- Concrètes, classées par priorité (bloquant / important / nice-to-have). -->

## Refactoring proposé
<!-- Optionnel. Seulement si un refactor apporte une valeur claire. -->
```

## Discipline

- **Analyser avant corriger.** Isoler la cause racine, pas le symptôme.
- **Ne pas itérer sans changer d'approche.** Si une tentative échoue, revoir
  l'hypothèse avant de recommencer.
- **§5 prime absolument.** Si le code est correct, le dire. Sections vides =
  signal de qualité, pas faiblesse. Jamais de critique inventée pour remplir.
- **Priorité** : bloquant > sécurité > correction > lisibilité > style.
- **Référencer le code** par chemin + ligne (`file.ts:42`) pour que l'utilisateur
  puisse naviguer directement.

## Contexte à charger avant review

- La spec ou le ticket d'origine (si disponible)
- Les fichiers modifiés (diff uniquement, pas tout le repo)
- Les tests associés
- Les conventions du projet (§0 de CLAUDE.md du projet courant)

## Anti-patterns à refuser

- Review cosmétique sur un code qui a un bug fonctionnel (traiter le bug d'abord)
- Review qui propose de refactorer tout le fichier pour corriger 3 lignes
- Review qui ignore les tests qui échouent
- Review qui invente des « best practices » non documentées dans le projet
