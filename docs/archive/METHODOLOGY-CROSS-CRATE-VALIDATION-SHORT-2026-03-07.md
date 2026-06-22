# Méthodologie compacte — validation Rust cross-crates

Date : 2026-03-07  
Auteur : GitHub Copilot  
Destinataire : Claude

## Position

Le bon compromis n’est pas de choisir entre validation **par crate** et validation **workspace**.
Il faut les utiliser à des moments différents.

- **Pendant le dev itératif** : valider rapidement par crate et par downstream direct.
- **Avant commit/push final** : valider la surface CI réelle, idéalement jusqu’au `workspace`.

## Règle simple

Un patch Rust cross-crates n’est **pas terminé** tant que :

1. le crate touché compile,
2. le downstream principal compile,
3. les tests du crate touché compilent,
4. `clippy -D warnings` passe sur la surface CI réellement concernée,
5. et, avant push final, la validation globale pertinente a été exécutée.

## Workflow recommandé

### Boucle itérative minimale

```text
cargo build -p <crate touché>
cargo build -p <crate downstream>
cargo test -p <crate touché> --lib --no-run
cargo clippy -p <crate touché> -- -D warnings
```

Si la CI cible déjà un autre crate dépendant important :

```text
cargo clippy -p <crate CI concerné> -- -D warnings
```

## Validation finale avant push

Pour un patch cross-crates significatif, surtout sur discovery / relay / gossip :

```text
cargo test --workspace
cargo clippy --workspace -- -D warnings
```

## Règle spécifique discovery / relay

Pour tout patch touchant discovery / relay / gossip / transport / runtime :

- cartographier les crates impactés avant code,
- vérifier les surfaces `relay`, `discovery`, `stress`,
- ne jamais supposer que seul le crate édité compte.

## Application au cas PeerPresent

Sur ce sujet, les surfaces minimales pertinentes sont :

- `tom-connect`
- `tom-transport`
- `tom-protocol`
- parfois `tom-stress`

## Doctrine finale

> En itératif, on valide vite par crate.  
> Avant de pousser, on valide large par surface CI réelle, et si le patch est assez transversal, par `workspace`.

C’est la façon la plus fiable d’éviter un patch “ça compile localement” mais “ça casse encore en CI”.
