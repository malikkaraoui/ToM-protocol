# Xcode Integration Guide — tom-relay FFI pour tvOS

## État actuel

✅ **Code prêt à l'intégration**:
- Rust FFI library compilée : `build/libtom_relay_ffi.a` (53 MB)
- C header : `build/TomRelayFFI.h`
- Swift code mis à jour : `RelayManager.swift` (appels FFI réels)
- Bridging header : `TomRelay-Bridging-Header.h`

## Étapes d'intégration dans Xcode

### 1. Ouvrir le projet Xcode

```bash
cd /Users/malik/Documents/tom-protocol/apps/relay-tvos/TomRelay
open TomRelay.xcodeproj
```

### 2. Ajouter la bibliothèque statique

1. Dans Xcode, sélectionner le **target TomRelay**
2. Aller dans **Build Phases** → **Link Binary With Libraries**
3. Cliquer sur **+** et **Add Other...** → **Add Files...**
4. Naviguer vers `/Users/malik/Documents/tom-protocol/apps/relay-tvos/build/libtom_relay_ffi.a`
5. Cliquer **Add**

### 3. Configurer le bridging header

1. Sélectionner le **target TomRelay**
2. Aller dans **Build Settings**
3. Chercher **Objective-C Bridging Header**
4. Définir la valeur à : `TomRelay/TomRelay-Bridging-Header.h`

### 4. Ajouter le header search path

1. Dans **Build Settings**, chercher **Header Search Paths**
2. Ajouter : `$(PROJECT_DIR)/../../build` (non-recursive)

Ou directement le chemin absolu:
```
/Users/malik/Documents/tom-protocol/apps/relay-tvos/build
```

### 5. Vérifier les fichiers du projet

Dans le navigateur de projet Xcode, vérifier que ces fichiers sont présents :
- `TomRelay/ContentView.swift` ✓
- `TomRelay/RelayManager.swift` ✓ (mis à jour avec FFI)
- `TomRelay/TomRelayApp.swift` ✓
- `TomRelay/TomRelayFFI.h` (à ajouter manuellement si absent)
- `TomRelay/TomRelay-Bridging-Header.h` ✓

Si `TomRelayFFI.h` n'est pas dans le projet:
1. Drag & drop `build/TomRelayFFI.h` dans le dossier `TomRelay/` du navigateur Xcode
2. Cocher **Copy items if needed**

### 6. Build et test

#### Simulateur tvOS

1. Sélectionner **Any tvOS Simulator** comme destination
2. Compiler avec `Product` → `Build` (Cmd+B)
3. Si le build échoue avec une erreur de linkage :
   - Rebuild la bibliothèque pour simulateur :
     ```bash
     TVOS_TARGET=aarch64-apple-tvos-sim \
     ./scripts/build-apple-tv-relay-ffi.sh
     ```
   - Re-linker la nouvelle bibliothèque dans Xcode (étapes 2)

4. Lancer avec `Product` → `Run` (Cmd+R)

#### Apple TV HD physique

1. Brancher l'Apple TV HD via USB-C (si disponible) ou WiFi pairing
2. Sélectionner **Apple TV HD** comme destination
3. Signer le build avec votre Apple Developer account (dans **Signing & Capabilities**)
4. Compiler et lancer (Cmd+R)

### 7. Validation

Une fois l'app lancée sur l'Apple TV :

1. Appuyer sur **Start** → Le relay doit démarrer
2. Le status doit passer de "Stopped" à "Running" (ou "Initializing server...")
3. Depuis le MacBook Pro, tester :

```bash
# Remplacer <IP_APPLE_TV> par l'IP de l'Apple TV (visible dans Réglages → Réseau)
curl http://<IP_APPLE_TV>:3343/health
```

Si le relay tourne, vous devriez recevoir une réponse HTTP 200.

## Troubleshooting

### Erreur: "Use of unresolved identifier 'tom_relay_start'"

→ Le bridging header n'est pas configuré ou le header FFI n'est pas trouvé.

**Fix**:
- Vérifier que `TomRelay-Bridging-Header.h` est bien configuré dans Build Settings
- Vérifier que `TomRelayFFI.h` est dans le Header Search Path

### Erreur de linkage: "Undefined symbols for architecture arm64"

→ La bibliothèque statique n'est pas linkée ou compilée pour la mauvaise architecture.

**Fix simulateur**:
```bash
TVOS_TARGET=aarch64-apple-tvos-sim ./scripts/build-apple-tv-relay-ffi.sh
```

**Fix device**:
```bash
TVOS_TARGET=aarch64-apple-tvos ./scripts/build-apple-tv-relay-ffi.sh
```

### Le relay démarre mais ne répond pas au curl

→ Firewall tvOS ou problème réseau LAN.

**Fix**:
- Vérifier que l'Apple TV et le Mac sont sur le même réseau WiFi
- Tester d'abord sur simulateur (127.0.0.1:3343)
- Vérifier les logs dans la Console tvOS (Window → Devices and Simulators → Apple TV → View Logs)

## Next Steps (Optionnel)

### Ajouter un timer pour rafraîchir le status

Dans `RelayManager.swift`, ajouter un timer qui appelle `updateStatus()` toutes les 5 secondes :

```swift
private var statusTimer: Timer?

func start() {
    // ... existing code ...

    // Start status polling
    statusTimer = Timer.scheduledTimer(withTimeInterval: 5.0, repeats: true) { [weak self] _ in
        self?.updateStatus()
    }
}

func stop() {
    // ... existing code ...

    // Stop status polling
    statusTimer?.invalidate()
    statusTimer = nil
}
```

### Afficher les métriques Prometheus

Ajouter un deuxième endpoint dans `ContentView.swift` pour voir les métriques :

```swift
Text("Metrics: http://\(manager.bindAddress.replacingOccurrences(of: "3343", with: "9093"))/metrics")
    .font(.caption)
```

## Prochaine étape : Freebox V6 + MacBook Air

Une fois l'Apple TV validée, reproduire le process pour :
1. **MacBook Air** (via SSH ou USB bundle install)
2. **Freebox V6** (cross-compile ARM 32-bit, SSH deploy)

Objectif : réseau local 3-4 relays pour validation stress tests.
