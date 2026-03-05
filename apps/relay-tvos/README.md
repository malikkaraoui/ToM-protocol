# relay-tvos starter

Starter minimal pour intégrer `tom-relay` dans une app tvOS.

## Objectif

Avoir une app Apple TV qui lance/arrête le relay depuis l'UI, avec logs d'état.

## Pré-requis

- Xcode installé (tvOS SDK)
- Compte Apple Developer configuré dans Xcode
- Apple TV appairée au Mac
- Rust target tvOS installé

## Important

Sur tvOS, on ne peut pas exécuter arbitrairement un binaire externe comme sur Linux/macOS.
Le chemin réaliste est :

1. Compiler du code Rust compatible tvOS (idéalement en bibliothèque FFI)
2. Lier cette bibliothèque dans l'app tvOS
3. Piloter le runtime relay via fonctions exposées (`tom_relay_start`, `tom_relay_stop`)

Le dossier `TomRelay/` contient un squelette Swift prêt à être collé dans un projet Xcode tvOS.

## Workflow rapide

1. Créer un projet tvOS App dans Xcode nommé `TomRelay`
2. Ajouter les fichiers Swift de `apps/relay-tvos/TomRelay/`
3. Ajouter le header C `TomRelayFFI.h` au bridging header
4. Construire les artefacts Rust tvOS (script ci-dessous)
5. Lier la lib Rust dans Xcode (Build Phases > Link Binary With Libraries)
6. Lancer sur Apple TV

## Pas à pas (débutant)

### A. Préflight outils

```bash
./scripts/apple-tv-preflight.sh
```

### B. Build en simulateur d'abord (recommandé)

```bash
TVOS_TARGET=aarch64-apple-tvos-sim ./scripts/build-apple-tv-relay.sh
```

Artefact attendu : `apps/relay-tvos/build/tom-relay-aarch64-apple-tvos-sim`

### C. Appairer l'Apple TV physique

1. Apple TV → **Settings** → **Remotes and Devices** → **Remote App and Devices**
2. Sur Mac : ouvrir Xcode
3. **Window** → **Devices and Simulators**
4. Sélectionner l'Apple TV et valider le code de pairing

Validation CLI :

```bash
xcrun devicectl list devices
```

### D. Build cible Apple TV physique

```bash
./scripts/build-apple-tv-relay.sh
```

> Si le build device échoue mais le simulateur passe, continuer l'intégration UI et pairing ; le point bloquant est côté linkage binaire tvOS, pas côté Swift/Xcode.

## Scripts repo

- `scripts/apple-tv-preflight.sh` : vérifie l'environnement Xcode/Rust tvOS
- `scripts/build-apple-tv-relay.sh` : build Rust tvOS + copie artefacts dans `apps/relay-tvos/build/`

## Critère GO

Depuis ton MacBook Pro :

- `curl http://<IP_APPLE_TV>:3343/health`

Doit répondre HTTP 200.
