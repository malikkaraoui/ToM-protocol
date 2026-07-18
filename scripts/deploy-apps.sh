#!/usr/bin/env bash
# Déploie les apps ToM (build courant du XCFramework FFI) sur les appareils réels.
# Pré-requis : scripts/build-tom-protocol-ffi-xcframework.sh déjà lancé
# (DEVICE_ONLY=1 pour n'avoir que les slices device iOS/tvOS).
#
# Étapes par appareil : sync xcframework → xcodebuild → devicectl install → launch.
# Un appareil injoignable (unavailable) est SAUTÉ, pas fatal (ex : iPhone en 4G).
#
# Projet UNIQUE (apps/tom-node-tvos, target "TomNode") — gère tvOS + iOS/iPadOS
# dans le même target universel (SUPPORTED_PLATFORMS: appletvos + iphoneos).
# Un ancien scaffold iOS séparé (apps/tom-node-ios) a coexisté un temps sans
# jamais être supprimé après cette unification — ce script le reconstruisait
# activement à chaque déploiement, causant deux apps installées en parallèle
# sur le même appareil (même bundle ID, icônes différentes). Supprimé 2026-07-13.
#
# Usage : bash scripts/deploy-apps.sh
set -u
cd "$(dirname "$0")/.."

IPAD=1DC13ED3-A246-525E-8B71-7F62AA950A4A          # iPad Air M4 (iOS 26.5)
APPLETV=2C36638F-EC0B-5A75-B74F-5507E15D1E98       # Apple TV "Séjour"
IPHONE=C3CFC878-6C8D-5B87-8B35-1CB108A4922A        # iPhone 12 Pro (iOS 18.7.2)
IPHONE_LAURA=5F7DA86D-D802-5C1B-9180-2950A7DC1306  # iPhone 12 Pro "Laura" (ajouté 18/07)

PROJ=apps/tom-node-tvos/TomNode.xcodeproj
# Bundle UNIQUE depuis l'unification (le target universel garde l'id tvOS).
# L'ancien id "…TomNode-iOS" faisait échouer le launch iPad (install OK, launch unknown).
IOS_BUNDLE=malik.karaoui.TomNode
TV_BUNDLE=malik.karaoui.TomNode
IOS_APP=build/ios-app/Build/Products/Debug-iphoneos/TomNode.app
TV_APP=build/tvos-app/Build/Products/Debug-appletvos/TomNode.app

ts() { date "+%H:%M:%S"; }
log() { echo "[$(ts)] $*"; }

# Validation de ONLY AVANT tout travail coûteux (le sync XCFramework prend du
# temps) : un ONLY mal orthographié ne doit pas "réussir" en ne déployant rien.
# Mesuré avant ce garde-fou : ONLY=iphone-lora sortait en EXIT=0 sans toucher un
# seul appareil — le faux positif silencieux qui fait croire à un déploiement.
DEVICES="ipad appletv iphone iphone-laura"
ONLY="${ONLY:-}"
if [ -n "$ONLY" ]; then
  case " $DEVICES " in
    *" $ONLY "*) ;;
    *) log "❌ ONLY='$ONLY' inconnu — valeurs possibles : $DEVICES"; exit 2 ;;
  esac
fi

# Un appareil est-il joignable ? (connected|available dans devicectl)
reachable() {
  xcrun devicectl list devices 2>/dev/null | grep "$1" | grep -qiE "connected|available"
}

# build (iOS) + install + launch sur un device iOS donné.
deploy_ios() {
  local dev="$1" name="$2"
  if ! reachable "$dev"; then log "[skip] ${name} (${dev}) injoignable — sauté"; return 0; fi
  log "[build] iOS pour ${name} ..."
  xcodebuild -project "$PROJ" -scheme TomNode -configuration Debug \
    -destination "id=${dev}" -derivedDataPath build/ios-app \
    -allowProvisioningUpdates build >"/tmp/xcb-ios-${name}.log" 2>&1
  if [ $? -ne 0 ]; then log "[FAIL] build iOS ${name} (voir /tmp/xcb-ios-${name}.log)"; tail -5 "/tmp/xcb-ios-${name}.log"; return 1; fi
  log "[install] ${name} ..."
  xcrun devicectl device install app --device "${dev}" "$IOS_APP" >"/tmp/inst-${name}.log" 2>&1 \
    && log "[launch] ${name}" && xcrun devicectl device process launch --device "${dev}" "$IOS_BUNDLE" >/dev/null 2>&1 \
    && log "[OK] ${name} déployé" \
    || { log "[FAIL] install/launch ${name}"; tail -5 "/tmp/inst-${name}.log"; return 1; }
}

deploy_tvos() {
  local dev="$1"
  if ! reachable "$dev"; then log "[skip] Apple TV injoignable — sauté"; return 0; fi
  log "[build] tvOS ..."
  xcodebuild -project "$PROJ" -scheme TomNode -configuration Debug \
    -destination "id=${dev}" -derivedDataPath build/tvos-app \
    -allowProvisioningUpdates build >/tmp/xcb-tvos.log 2>&1
  if [ $? -ne 0 ]; then log "[FAIL] build tvOS (voir /tmp/xcb-tvos.log)"; tail -5 /tmp/xcb-tvos.log; return 1; fi
  log "[install] Apple TV ..."
  xcrun devicectl device install app --device "${dev}" "$TV_APP" >/tmp/inst-tvos.log 2>&1 \
    && log "[launch] Apple TV" && xcrun devicectl device process launch --device "${dev}" "$TV_BUNDLE" >/dev/null 2>&1 \
    && log "[OK] Apple TV déployée" \
    || { log "[FAIL] install/launch Apple TV"; tail -5 /tmp/inst-tvos.log; return 1; }
}

# ── Sync du XCFramework fraîchement buildé vers le package ──
log "🔄 sync XCFramework → package"
bash scripts/sync-xcframework-to-package.sh || { log "❌ sync échoué — as-tu buildé le FFI ?"; exit 1; }

# ONLY=<nom> déploie un seul appareil (ex : ONLY=iphone-laura bash scripts/deploy-apps.sh).
# Utile pour intégrer un nouvel appareil sans redéployer — ni réveiller — toute la flotte.
#
# Un ONLY mal orthographié ne doit PAS "réussir" en ne déployant rien : c'est le
# faux positif silencieux qui fait croire à un déploiement (mesuré : ONLY=iphone-lora
# sortait en EXIT=0 sans toucher un seul appareil). D'où la validation ci-dessous.
want() { [ -z "$ONLY" ] || [ "$ONLY" = "$1" ]; }

# Un échec de déploiement doit remonter dans le code de sortie, sinon un
# xcodebuild rouge passe pour un succès (cf. build-swift-masqué-par-deriveddata).
failed=""
try() { local name="$1"; shift; want "$name" || return 0; "$@" || failed="$failed $name"; }

log "═══ DÉPLOIEMENT${ONLY:+ (ONLY=$ONLY)} ═══"
try ipad         deploy_ios  "$IPAD" ipad
try appletv      deploy_tvos "$APPLETV"
try iphone       deploy_ios  "$IPHONE" iphone
try iphone-laura deploy_ios  "$IPHONE_LAURA" iphone-laura
if [ -n "$failed" ]; then
  log "═══ FIN déploiement — ÉCHECS :$failed ═══"
  exit 1
fi
log "═══ FIN déploiement (OK) ═══"
