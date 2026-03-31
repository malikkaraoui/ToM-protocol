# PeerPresent `k` presets

Presets prêts pour comparer le fanout relay-assisted :

- `tom-relay-k8.toml`
- `tom-relay-k16.toml`
- `tom-relay-k32.toml`

## Lancer le relay

Depuis la racine du repo :

- `cargo run -p tom-relay -- --dev --config-path deploy/peerpresent-k/tom-relay-k8.toml`
- `cargo run -p tom-relay -- --dev --config-path deploy/peerpresent-k/tom-relay-k16.toml`
- `cargo run -p tom-relay -- --dev --config-path deploy/peerpresent-k/tom-relay-k32.toml`

## Campagne recommandée

Pour chaque preset :

1. démarrer le relay avec le preset choisi ;
2. lancer le même scénario côté nodes ;
3. relever :
   - temps jusqu'au premier `GossipNeighborUp` ;
   - temps jusqu'au premier message livré ;
   - taux de succès bootstrap ;
   - éventuels logs `peer_present dropped: channel full`.

## Notes

- `--dev` garde le relay en HTTP simple sur le port dev si rien d'autre n'est précisé.
- Le fichier TOML ne change ici que `peer_present_k` pour isoler la variable mesurée.
- Voir aussi `docs/plans/2026-03-30-peerpresent-k-measurement.md` pour le protocole de comparaison.
