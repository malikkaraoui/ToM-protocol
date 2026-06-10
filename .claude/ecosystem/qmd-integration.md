---
kind: ecosystem
name: qmd-integration
loads_from: src/fr/CLAUDE.md (header + §19)
volatile: true
see_also: docs/qmd-user-guide.md (guide utilisateur complet, non charge en runtime)
---

# Ecosystem — QMD Integration

> Chargé à la demande. Définit **comment Claude doit utiliser QMD** quand
> l'outil est actif dans le projet. Le guide d'installation/config complet
> est dans `docs/qmd-user-guide.md` (non chargé par Claude au runtime).

## Qu'est-ce que QMD ?

QMD est un moteur de recherche hybride local (BM25 + embeddings +
reranking) indexant des fichiers Markdown. Il est exposé à Claude via MCP.

**Cas d'usage principal de l'utilisateur** : pipeline Claude ↔ Copilot via
fichiers `.md` (plans, bugs, décisions) — QMD retrouve le bon fichier
avant de relancer un raisonnement, évitant de reproduire les mêmes erreurs.

## Quand Claude doit interroger QMD

- **Reprise de bug** : avant d'analyser un bug, chercher si un bug similaire
  a déjà été documenté et résolu (`qmd query "description du bug"`)
- **Reprise de plan** : avant de planifier une feature, chercher les plans
  existants sur le même sujet
- **Doute sur une convention** : chercher si une décision a été prise dans
  une session précédente
- **Contexte volumineux** : au lieu de charger 20 fichiers spéculativement,
  chercher les 3-4 les plus pertinents

## Quand NE PAS interroger QMD

- **Question factuelle triviale** : l'overhead de la recherche > le gain
- **Info qui doit être fraîche** : QMD est une photo du dernier `qmd embed`,
  pas du code courant. Pour du code, lire directement le fichier.
- **QMD n'est pas configuré dans le projet courant** : regarder `§0` pour
  savoir s'il est actif avant de l'invoquer

## Pattern d'invocation typique

```bash
# Recherche hybride (recommandee par defaut)
qmd query "auth middleware bug token expiry" --files --min-score 0.4 -n 5

# Restreinte a une collection
qmd query "BMAD phase 2" -c workspace --files

# Avec contenu complet pour extraction directe
qmd query "emergency secret rotation" --full -n 3
```

Paramètres utiles :

- `--files` : sortie chemins, idéal pour injection directe dans le contexte
- `--min-score 0.4` : seuil de pertinence raisonnable (sous 0.3, c'est du
  bruit)
- `-c <collection>` : restreint à une collection (plus précis + plus rapide)
- `-n <num>` : limite le nombre de résultats (évite de saturer la fenêtre)

## Discipline

- **Toujours filtrer par score** : résultats < 0.3 à ignorer, 0.3-0.5 à
  vérifier manuellement, > 0.5 à exploiter
- **Ne jamais charger tous les résultats à l'aveugle** : lire le score et
  l'extrait, sélectionner 2-3 fichiers max
- **Préférer la recherche hybride (`qmd query`)** à BM25 pur (`qmd search`)
  ou vectoriel pur (`qmd vsearch`) sauf cas spécifique
- **Lister QMD dans `§0` MCPs actifs** pour tracer son usage par projet

## Anti-patterns

- Invoquer QMD systématiquement pour chaque question, même triviale
- Charger tous les résultats (`--all`) par défaut → saturation fenêtre
- Traiter un résultat 0.2 comme pertinent
- Utiliser QMD pour remplacer la lecture directe du code courant
