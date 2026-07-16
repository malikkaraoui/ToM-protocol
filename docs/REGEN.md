# REGEN — Restauration après sauvegarde disque (2026-06-14)

> Ce dépôt a été **nettoyé pour export disque dur** le 2026-06-14 : **73G → 652M**.
> Tout ce qui a été supprimé est **régénérable**. Le code source, `deploy/` et la config sont intacts.
> Lis ce fichier en premier au redémarrage du projet.

## Ce qui a été supprimé (régénérable, AUCUNE perte de code)

| Catégorie | Chemins | Régénéré par |
|---|---|---|
| Builds Rust | `target/`, `crates/tom-protocol-ffi/target/`, `experiments/iroh-poc/target/` | `cargo build` |
| Builds Swift | tous les `**/.build/` | `swift build` / `xcodebuild` |
| Node | tous les `node_modules/` | `pnpm install` |
| XCFramework FFI | `sdk/swift/TomProtocolKit/Artifacts/TomProtocolFFI.xcframework`, `apps/tom-node-tvos/build/` | rebuild FFI multi-slices (voir §Rebuild XCFramework) |
| Cruft mort | backup `.claude-backup-*`, worktrees orphelins `.claude/worktrees/*`, `logs/*`, `.DS_Store`, `.bak` | non régénéré (déchet pur) |

**Gardé volontairement** : tout le code source, `deploy/` (binaires .app/relay/stress prêts à redéployer, 106M), `graphify-out/` (tracké).

## Séquence de régénération complète

```bash
cd "/Users/malik/Documents/ATELIER PROJETS/tom-protocol"

# 1. Dépendances JS (réactive husky/biome, hooks pre-commit)
pnpm install

# 2. Build + test Rust workspace
cargo build --workspace
cargo test --workspace

# 3. Build + test TypeScript (legacy)
pnpm build && pnpm test

# 4. (si dev Apple) rebuild XCFramework FFI — voir section suivante
```

## Rebuild XCFramework (iOS / tvOS / macOS)

Le XCFramework `TomProtocolFFI` (5 slices : ios-arm64, ios-arm64-simulator, tvos-arm64,
tvos-arm64-simulator, macos-arm64_x86_64) se reconstruit depuis `crates/tom-protocol-ffi`.
Chercher le script de packaging (probablement dans `scripts/` ou `sdk/swift/`) :

```bash
ls scripts/ | grep -iE "xcframework|ffi|build-apple"
ls sdk/swift/
```

Puis lancer ce script ; il recompile la lib Rust pour chaque cible Apple et assemble le
`.xcframework` dans `sdk/swift/TomProtocolKit/Artifacts/`. Build long (multi-cibles).

## État git au moment de la sauvegarde

- Branche `main`, dernier commit poussé : `5be51f9` (feat dashboard + clean-cruft.sh v1).
- **Non poussé** (présent dans le working tree / cette sauvegarde) :
  - `scripts/clean-cruft.sh` — **v2** (ajoute la purge `node_modules`/`DerivedData` au mode `--builds`).
    Non poussé volontairement : la gate pre-push relance un smoke-test qui **reconstruirait `target/`**
    et regonflerait le dossier juste avant l'export. À committer/pousser au prochain rebuild.
  - `graphify-out/GRAPH_REPORT.md`, `graphify-out/graph.json` — régénérés par un hook, non critiques.

## Outil de nettoyage réutilisable

`scripts/clean-cruft.sh` — anti-empilement disque, dry-run par défaut :

```bash
bash scripts/clean-cruft.sh                  # dry-run, cruft mort seulement
bash scripts/clean-cruft.sh --apply          # exécute (cruft mort ~découplé des builds)
bash scripts/clean-cruft.sh --apply --builds # + tous les artefacts régénérables (target, .build, node_modules)
```

Règles : backups datés par leur **nom** (purge > 7j, garde le récent) · worktrees supprimés
seulement si **0 commit non mergé** · `--builds` séparé pour ne jamais purger les builds par accident.
