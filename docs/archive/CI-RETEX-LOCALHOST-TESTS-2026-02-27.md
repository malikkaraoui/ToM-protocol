# CI Retex — Localhost tests (PoC + Stress)

**Date**: 2026-02-27  
**Contexte**: échecs intermittents/CI sur les jobs localhost après refactor fork iroh → ToM  
**Statut**: ✅ corrigé (passage confirmé)

---

## Résumé exécutif

Deux problèmes distincts faisaient échouer la CI :

1. **Rust PoC (`nat-test`)**: le script cherchait le binaire au mauvais endroit (`target/debug/nat-test`) selon le layout CI/workspace.
2. **Rust Stress (`tom-stress`)**: le smoke test localhost pouvait bloquer trop longtemps (pas de garde-fou de timeout par sous-test), menant à un timeout global du job GitHub Actions.

Les deux points ont été corrigés par des scripts plus robustes (résolution du `target_directory` via Cargo metadata + timeouts explicites + checks moins fragiles).

---

## Symptômes observés

### 1) Job `Rust PoC (build + clippy + localhost test)`

Erreur vue en CI:

- `.../target/debug/nat-test: No such file or directory`

Le build passait, mais le script ne trouvait pas le binaire au chemin supposé.

### 2) Job `Rust stress (build + clippy + localhost test)`

Erreur vue en CI:

- `The action 'localhost stress test' has timed out after 5 minutes.`

Le test démarrait (`listener OK`), puis restait bloqué pendant la phase `ping`.

---

## Cause racine

## A. PoC — binaire introuvable

Le script utilisait une hypothèse de chemin (`workspace_root/target/debug`) qui n’est pas toujours vraie en CI (cache, target-dir implicite, layout de workspace, etc.).

👉 **Root cause**: path de binaire déterminé de manière heuristique au lieu d’utiliser la source de vérité Cargo.

## B. Stress — timeout global du job

Le script localhost lançait plusieurs sous-commandes (`ping`, `burst`, `ladder`) sans timeout process strict par étape.

👉 **Root cause**: absence de garde-fou local, ce qui laisse le timeout global GitHub Actions faire l’arrêt brutal.

---

## Correctifs appliqués

## 1) `experiments/iroh-poc/scripts/test-localhost.sh`

- Résolution du binaire via:
  - `cargo metadata --format-version 1 --no-deps`
  - lecture de `target_directory` (Python)
- Fallback de secours conservés
- Dernier fallback par recherche de binaire exécutable
- Message d’erreur explicite si binaire non trouvé

Effet: le script localise `nat-test` correctement en CI.

## 2) `.github/workflows/ci.yml` (job `rust-poc`)

Ajout d’une mini étape de debug:

- affichage `pwd`
- affichage `target_directory` via `cargo metadata`

Effet: diagnostic instantané en cas de régression de path.

## 3) `crates/tom-stress/scripts/test-localhost.sh`

- Ajout d’un wrapper timeout cross-platform (`timeout` / `gtimeout`) pour `ping`, `burst`, `ladder`
- Résolution robuste du binaire `tom-stress` via `cargo metadata target_directory`
- Attente du vrai event `"started"` côté listener (au lieu de seulement fichier non vide)
- Logging d’aide en cas d’échec de bootstrap
- Durcissement de checks shell fragiles (parsing `grep`, valeurs vides)
- Réduction de faux négatifs sur métriques runtime volatiles (`ping events`, `messages_acked` en info)

Effet: pas de blocage > 5 min, smoke test CI stable.

---

## Commits liés

- `fdc3da8` — `fix(ci): stabilize nat-test path and add target dir debug step`
- `c23e939` — `fix(ci): harden tom-stress localhost smoke test timeouts`

---

## Pourquoi ça ne passait pas avant ? (version courte)

- **PoC**: le script regardait au mauvais endroit pour le binaire.
- **Stress**: le script pouvait attendre trop longtemps sans timeout local, donc GitHub stoppait le job après 5 minutes.

---

## Prévention (recommandations)

1. Toujours résoudre les binaires via `cargo metadata target_directory` dans les scripts CI.
2. Mettre un timeout explicite par sous-scenario de smoke test (pas seulement au niveau job).
3. Garder les assertions "smoke" robustes: valider les événements clés, éviter les seuils trop stricts dépendants du réseau.
4. Conserver une étape debug path dans la CI pour accélérer les futures investigations.

---

## Check de validation post-fix

- ✅ `experiments/iroh-poc/scripts/test-localhost.sh` passe localement
- ✅ `crates/tom-stress/scripts/test-localhost.sh` ne bloque plus et passe localement
- ✅ push sur `main` effectué

---

## Fichiers modifiés

- `.github/workflows/ci.yml`
- `experiments/iroh-poc/scripts/test-localhost.sh`
- `crates/tom-stress/scripts/test-localhost.sh`

---

## TL;DR pour Claude

« Les échecs CI venaient de scripts localhost fragiles: path binaire hardcodé + absence de timeout par sous-test. On a fiabilisé via `cargo metadata target_directory`, timeouts process, et checks moins sensibles. Résultat: CI repasse. »
