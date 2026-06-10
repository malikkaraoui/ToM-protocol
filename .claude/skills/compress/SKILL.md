---
name: compress
description: "Compresse CLAUDE.md (et optionnellement les satellites) pour réduire les tokens input à chaque message. Compression prose uniquement — code, chemins, commandes intouchables."
figure: Intendant
---

# Compress — Réduction tokens input

> L'Intendant pèse chaque mot du manifeste. Ce qui ne porte pas de sens
> est supprimé. Ce qui reste pèse moins lourd à chaque message.

Compression syntaxique de CLAUDE.md. Code et chemins intouchables.

## Quand utiliser

- Avant un night-mode long (économie cumulée sur 8h)
- Après un refactor majeur qui a gonflé le fichier
- Si `npm run lint` indique > 140/150 lignes
- Sur demande explicite : `/compress`

## Ce qui est compressé / préservé

| Élément | Action |
| --- | --- |
| Articles (le, la, les, un, une, des) | Supprimés si non ambigus |
| Filler (simplement, vraiment, bien sûr, en fait, donc) | Supprimés |
| Hedges (il est possible que, on pourrait dire que) | Supprimés |
| Phrases complètes redondantes | Fusionnées en fragments |
| Blocs de code (``` ... ```) | Intouchables |
| Chemins, URLs, commandes | Intouchables |
| Noms de sections (§0, §1...) | Intouchables |
| Tableaux | Prose compressée, structure conservée |
| Règles absolues (§5, §22) | Compression minimale — sens primaire |

## Procédure

### Étape 1 — Backup

```bash
cp CLAUDE.md CLAUDE.md.bak
```

### Étape 2 — Analyser

Lire CLAUDE.md. Identifier :
- Les blocs prose (compressibles)
- Les blocs code/backtick (intouchables)
- Les lignes mixtes (compresser la prose, pas le code)

### Étape 3 — Compresser

Réécrire chaque bloc prose en mode fragment :
- Supprimer articles non ambigus
- Supprimer filler et hedges
- Fusionner phrases redondantes
- Garder l'intention exacte

Exemples :
```
Avant : "Il est important de noter que les fichiers sensibles doivent toujours être exclus"
Après : "Fichiers sensibles : toujours exclus."

Avant : "En début de session, signaler le modèle actif et recommander..."
Après : "Début session : signaler modèle actif, recommander..."
```

### Étape 4 — Vérifier

```bash
npm run lint
```

Doit rester ≤ 150 lignes. Si > 150 → compresser davantage avant d'écrire.

### Étape 5 — Rapport

"Compressé : X → Y lignes (-Z%). Backup : CLAUDE.md.bak.
Tokens input économisés par session : ~[estimation]."

## Règles

- Jamais modifier la sémantique — compression syntaxique uniquement
- Si ambiguïté sur le sens → garder l'original
- Ne pas descendre sous 80% du contenu original (risque de perte)
- Backup obligatoire avant toute écriture
- Ne compresser que CLAUDE.md par défaut ; satellites sur demande explicite
