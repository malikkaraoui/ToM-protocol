---
date: 2026-10-17
revue: R003
mandat: M0005
branche: docs/qmd-imperatif
verdict: GO
tip: 4d269b9e3911a4d5be3305a030850dd93f82d0ac
niveau: N2
---

# R003 — Doublage N2 QMD impératif

Périmètre : commit `4d269b9` (HEAD, worktree `/Users/malik/Documents/tom-protocol/.claude/worktrees/F02-R003`, base `127255e`). Lecture seule. `git status --short` vide avant revue.

## 1. Périmètre du diff : conforme

Commande : `git diff 127255e..HEAD --stat`
```
 .claude/CLAUDE.md | 2 +-
 CLAUDE.md         | 3 +++
 2 files changed, 4 insertions(+), 1 deletion(-)
```
Seuls `CLAUDE.md` et `.claude/CLAUDE.md` sont touchés. `git diff --check` : exit 0.

## 2. Bloc IMPÉRATIF en tête de CLAUDE.md : conforme

Preuve : `CLAUDE.md:1` = H1 `# ToM Protocol - Claude/LLM Documentation`, `CLAUDE.md:2` vide, `CLAUDE.md:3-4` = bloc (2 lignes, ≤ 4).
- Marqué IMPÉRATIF : `CLAUDE.md:3` « **IMPÉRATIF — QMD d'abord pour tout `.md`** ».
- Outils : `mcp__qmd__query` / `mcp__qmd__get` avant `Read` (`CLAUDE.md:3`).
- Read conditionné : « `Read` sur un `.md` seulement à ligne connue (offset + limit obligatoires) » (`CLAUDE.md:3`), fidèle à l'ancienne ligne.
- Longueur des lignes (python, caractères) : `CLAUDE.md:3` = 231, `CLAUDE.md:4` = 39 (≤ 400).
- Raison : `CLAUDE.md:4` « Raison : rapide, économe en contexte. » = 4 mots (≤ 6).
- Unicité : `grep -rn "QMD" CLAUDE.md .claude/CLAUDE.md` ne renvoie que `CLAUDE.md:3` et le renvoi `.claude/CLAUDE.md:88`. `grep "mcp__qmd__query"` ne renvoie que `CLAUDE.md:3`. La règle n'est pas dupliquée.
- Référence vérifiée : `.mcp.json` déclare bien le serveur `qmd` (`"qmd": {"command": "qmd", "args": ["mcp", "--index", "tom"]}`).

## 3. §15 de .claude/CLAUDE.md : remplacée, pas dupliquée

`git diff` : `.claude/CLAUDE.md:88` passe de l'ancienne ligne « **QMD-first** : pour tout `.md`, utiliser `mcp__qmd__get`/`mcp__qmd__query` avant `Read`. … » à :
`.claude/CLAUDE.md:88` = `**QMD-first** : voir l'impératif en tête du `CLAUDE.md` racine.` (63 caractères, une ligne, renvoi uniquement).

## 4. Aucun état ajouté : conforme

Les trois lignes ajoutées (`CLAUDE.md:3`, `CLAUDE.md:4`, `.claude/CLAUDE.md:88`) ne contiennent ni date, ni identifiant `Mxxxx`, ni compteur. Les seules dates présentes dans les fichiers (ex. `2026-07-20` dans `.mcp.json`) sont préexistantes et hors diff.

## 5. Budget : conforme

Commande : `wc -c` (octets) :
- `CLAUDE.md` : 32 302 (≤ 40 000)
- `.claude/CLAUDE.md` : 12 727 (≤ 40 000)

Caractères (python) : `CLAUDE.md` 31 094 → 31 367 (+273) ; `.claude/CLAUDE.md` 12 252 → 12 147 (−105). Croissance totale nette : +168 caractères (≤ ~300). Lignes ajoutées ≤ 400 caractères (max 231).

## 6. Markdown cohérent : conforme

- Titre H1 intact, bloc blockquote `CLAUDE.md:3-4` suivi d'une ligne vide `CLAUDE.md:5`, puis le paragraphe « This document provides… » inchangé.
- Aucune ligne préexistante modifiée dans `CLAUDE.md` (le diff n'a que des ajouts, 3 lignes).
- `.claude/CLAUDE.md` : une seule ligne remplacée, pas de rupture de section §15 (`:86` à `:90` intacts).

## Remarques (non bloquantes)

- Le bloc IMPÉRATIF dit « serveur `qmd` de `.mcp.json` » sans mentionner `--index tom`. Ce n'est pas exigé par le mandat ; l'étanchéité d'index est déjà documentée dans la mémoire `qmd-index-scope-tom`.
- Wording : le bloc racine dit « chercher et lire avec `mcp__qmd__query` / `mcp__qmd__get` », l'ancienne ligne disait « utiliser … avant `Read` ». Sens identique.
- L'auteur du commit est `Malik <karaoui.malik@gmail.com>` (conforme à la règle email gmail). Le message de commit ne porte pas de trailer `Co-Authored-By`, cohérent avec §13 (« jamais signer »).
- Remote : `git ls-remote origin docs/qmd-imperatif` = `4d269b9e…` = HEAD (aligné).

## Conclusion

Toutes les exigences 1 à 6 sont satisfaites, avec preuve. Aucun défaut prouvé. Verdict : **GO**.
