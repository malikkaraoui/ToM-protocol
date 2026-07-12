# TOM Protocol — Node Apple (tvOS + iOS + macOS)

> Révisé 2026-07-12 : ce document décrivait un plan "Phase 0 — pas encore démarré".
> C'était faux — tout le contenu ci-dessous (Phases 1-5) est **livré et fonctionnel**
> depuis plusieurs sessions, sous une architecture différente de celle planifiée
> (wrapper Swift local, pas de package `TomCoreKit` séparé). Voir décision
> `vault/20-decisions.md` (2026-06-07, "Garder le wrapper local").

## État réel

| Composant | Statut | Preuve |
|---|---|---|
| Xcode project (`apps/tom-node-tvos/TomNode.xcodeproj`) | ✅ livré | schemes `TomNode` (tvOS/iOS) + `TomNode-macOS` |
| `Makefile` (`apps/tom-node-tvos/Makefile`, PAS à la racine) | ✅ livré | `tvsim/tvrun/tvdevice/tvtest/macbuild/macrun/macdoctor/ffi-xcframework/devices/doctor` |
| FFI Rust (`crates/tom-protocol-ffi/{lib,types}.rs`) | ✅ livré | exclue du workspace (build `--locked`), `NodeStatusFFI` sérialisé serde (durci 2026-06-07) |
| XCFramework multi-plateforme | ✅ livré | `sdk/swift/TomProtocolKit/Artifacts/TomProtocolFFI.xcframework` — slices ios-arm64, ios-arm64-simulator, tvos-arm64, tvos-arm64-simulator, macos-arm64_x86_64 |
| Swift Package `TomProtocolKit` | ✅ livré | `sdk/swift/TomProtocolKit/` — MAIS ce n'est **pas** le `TomCoreKit` décrit plus bas (Phase 3.1) : c'est le conteneur de l'artefact XCFramework + `TomVersion.swift`, pas une couche d'abstraction Swift-native |
| Wrapper Swift (au lieu de `TomCoreKit` actor) | ✅ livré | `apps/tom-node-tvos/TomNode/Services/TomNodeService.swift` — wrapper direct sur le FFI, décision explicite de ne pas ajouter de couche `TomCoreKit` (elle ne débloquait rien) |
| UI SwiftUI (tvOS + iOS + macOS) | ✅ livré | `apps/tom-node-tvos/TomNode/Views/{ContentView,StatusView,MessagesView,GroupsView,SettingsView,LogView}.swift` |
| Build macOS réel testé | ✅ vérifié 2026-07-12 | `make macbuild` → `BUILD SUCCEEDED`, app lancée et confirmée process natif (`ps aux`), fermée proprement |
| Build tvOS/iOS device réel testé | ⚠️ non re-testé cette session | devices (iPad/iPhone/AppleTV) **Offline** dans `xcrun xctrace list devices` au 2026-07-12, blocage réseau/pairing, pas un problème de code |
| Test P2P multi-nœud réel (Mac ↔ device) | ⚠️ bloqué | NAS (relais, `192.168.0.21`) injoignable ce jour ; à refaire dès que flotte + NAS reviennent en ligne |
| `curl http://<device>:PORT/health` (relay embarqué) | non re-vérifié cette session | `StatusServer.swift` existe côté device (voir Services/) |

**Build Swift courant** : `TomVersion.build = 37` (dernier bump, post red-team L1-003 round 2).

## Écart architecture vs plan original

Le plan original (ci-dessous, conservé comme référence historique) prévoyait un package
Swift `Packages/TomCoreKit` avec un `actor TomNode` encapsulant le FFI, séparé de l'app.
Décision du 2026-06-07 (voir `vault/20-decisions.md`) : **ne pas créer ce package**. Le FFI
est câblé directement dans `apps/tom-node-tvos/TomNode/Services/TomNodeService.swift` via
un wrapper local (`TomNodeWrapper` actor + service). Raison : `TomCoreKit` n'aurait débloqué
aucun problème réel, le FFI était déjà entièrement fonctionnel via le wrapper existant —
introduire un package séparé aurait été de la sur-ingénierie sans bénéfice mesurable.

## Ce qui reste réellement ouvert

1. **Validation flotte réelle post-L1-003** (bloquée, pas un manque de code) — redéployer
   build 37+ sur iPad/iPhone/AppleTV/NAS dès que ces appareils/le NAS reviennent en ligne,
   confirmer send/receive 1-1 + groupe sur device physique avec le durcissement L1-003 actif.
2. **Contrainte iOS/tvOS suspension réelle** (limite OS inhérente, documentée dans
   `CLAUDE.md` §Known Limitations #4) — pas un chantier tvOS spécifique, s'applique à toute
   la flotte Apple.
3. Pas de gap de code connu identifié à ce jour dans le node Apple lui-même.

---

## Annexe — plan original (référence historique, phases toutes livrées)

Les sections suivantes sont l'archive du plan initial. Elles ne reflètent plus l'état du
code (ex : Phase 3.1 décrit `TomCoreKit`, abandonné — voir écart ci-dessus) mais sont
conservées pour l'historique de la décision.

### Phase 1 — Bootstrap projet Xcode ✅ livré (architecture réelle : voir tableau ci-dessus)
### Phase 2 — FFI `tom-protocol-ffi` ✅ livré (`crates/tom-protocol-ffi/`)
### Phase 3 — Bindings Swift ✅ livré, **sous forme de wrapper local**, pas `TomCoreKit`
### Phase 4 — UI tvOS/iOS/macOS SwiftUI ✅ livré (`TomNode/Views/`)
### Phase 5 — Intégration & build ✅ livré (`Makefile` dans `apps/tom-node-tvos/`)
### Phase 6 — Tests E2E réels sur device physique ⚠️ partiel (macOS ✅ vérifié, tvOS/iOS device ⚠️ bloqué par accès flotte, voir tableau)
### Phase 7 — Polish & doc ⚠️ ce document en fait partie (mise à jour 2026-07-12)

**Prepared by**: Claude Code
**Status révisé** : phases 1-5 livrées, phase 6 partielle (macOS vérifié, flotte tvOS/iOS bloquée par accès matériel), phase 7 en cours (ce doc)
