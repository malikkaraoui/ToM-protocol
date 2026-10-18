---
date: 2026-10-18
revue: R014
mandat: M0015
branche: chore/racine-et-pre-push-vault
verdict: GO
tip: 8210373750a7db63fde106a0cd6cb76181e7b562
niveau: N2
---

# Revue R014 — racine par PR + pre-push vault seul (#71)

Doublage N2 par sous-agents frais, en lecture seule (aucun commit, push ni cargo ; dépôts jetables sous `mktemp -d`). Trois passes, une par pointe : les deux premières ont prouvé chacune un fail-open, corrigé avant la suivante. Le verdict porte sur la pointe de code `8210373`.

## Passes

| Passe | Pointe | Verdict | Constat prouvé |
|-------|--------|---------|----------------|
| 1 | `191c0f1` | RESERVE | `printf "$_files" \| grep -qv '^vault/'` sous `pipefail` (`/bin/sh` macOS = bash 3.2, `.husky/pre-push:4`) : sur une liste > 64 Ko avec un chemin de code trié avant `vault/`, `printf` prend SIGPIPE, le pipeline vaut 141, le `return 1` ne part pas → CI sautée avec du code (8/8 runs). |
| 2 | `bf3548f` | RESERVE | `grep <<heredoc` puis `[ $? -eq 1 ]` : si le fichier temporaire du heredoc ne peut être créé (`ulimit -f 1`, SIGXFSZ ignoré), grep ne tourne pas, la redirection rend 1 → « vault seul » → CI sautée avec du code. |
| 3 | `8210373` | **GO** | Aucune faille prouvée. Test devenu purement en mémoire (`only_vault_paths` : IFS = saut de ligne, `set -f`, `case vault/*`), sans pipe ni heredoc. |

## Vérifications (passe 3)

| Point | Résultat |
|-------|----------|
| Suite `scripts/test-pre-push-vault.sh` | 16/16 OK, « TOUS LES TÉMOINS VERTS » |
| Hooks `191c0f1` et `bf3548f` contre la suite | chacun échoue au moins un témoin (> 64 Ko ; E/S bridées) |
| 6 cas obligatoires | vault seul → sautée ; code seul → rejouée ; vault + code → rejouée ; ref neuve vault seul → sautée ; suppression → sautée ; base introuvable → rejouée |
| Noms : espaces, `*`, `?`, `[x]`, `-x`, UTF-8, symlink | corrects ; `set -f` et IFS restaurés après la fonction |
| `Vault/`, `vaultx/`, `crates/vault-note.md`, `vault` sans slash | rejouée |
| Tabulation ou saut de ligne dans le nom | rejouée (git cite le nom) |
| Renommage `src/x → vault/x` | rejouée (`--no-renames`) |
| Heredoc externe des refs sous `ulimit -f` 1 à 600 | rejouée (`_seen=0`, échec fermé) |
| bash 3.2, `sh`, dash | conformes |
| Commit `bc973c8` (3 fichiers racine) | lignes ajoutées : aucune IP, aucun nom d'hôte, aucun secret (seuls des mots « secret » dans des commentaires de `pre-push-gate.sh`) ; `.claude/settings.json` JSON valide |
| Contrôle de dérive `ci-local.sh` | AWK_JOBS sur `ci.yml` moins `changes`/`harnais-ok` = tableau `JOBS`, même ordre ; pas de `paths:` |

## Remarques (non bloquantes → issues)

1. `diff.ignoreSubmodules=all` en config locale masque un gitlink modifié hors `vault/` (prouvé en dépôt jetable ; aucun sous-module dans le dépôt) → #74 (`--ignore-submodules=none`).
2. Témoins `Vault/` et `vaultx/` à ajouter à la suite → #74.
3. Comparaison d'arbres : « code puis revert » est sauté, comme le job `changes` → #74.
4. Job `changes` de `ci.yml` : grep en erreur → sortie vide → `code=false` (fail-open théorique) → #74.
5. `bc973c8` retire `"github@claude-plugins-official": true` de `.claude/settings.json` : c'est la version de la racine, enregistrée telle quelle sur décision du fondateur.
6. `vault/30-discoveries.md` contient des IP déjà présentes sur `origin/main` ; aucune ajoutée (suivi existant : #67).
7. Constat de session : `ci-local.sh` affiche « TOUT VERT. » quand il est tué par SIGTERM (code de sortie 143 conservé) → #73.
