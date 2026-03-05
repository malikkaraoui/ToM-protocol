#!/usr/bin/env bash
set -euo pipefail

# Prépare un dossier USB prêt à copier pour installer tom-relay sur MacBook Air.
#
# Usage:
#   ./scripts/prepare-usb-mac-air-relay.sh
#
# Optional (if already known):
#   AIR_USER=malik
#
# Optional:
#   AIR_RELAY_PORT=3341 AIR_METRICS_PORT=9091 AIR_COPY_STRESS=0
#   USB_OUT_DIR=/Volumes/MYUSB/tom-air-relay

AIR_USER="${AIR_USER:-REPLACE_USER}"

AIR_RELAY_PORT="${AIR_RELAY_PORT:-3341}"
AIR_METRICS_PORT="${AIR_METRICS_PORT:-9091}"
AIR_COPY_STRESS="${AIR_COPY_STRESS:-1}"
TEMPLATE_DIR="deploy/macos/usb-mac-air-relay"
USB_OUT_DIR="${USB_OUT_DIR:-target/usb-mac-air-relay-bundle}"

for cmd in cargo sed mkdir cp chmod; do
  if ! command -v "${cmd}" >/dev/null 2>&1; then
    echo "Missing required command: ${cmd}" >&2
    exit 1
  fi
done

echo "[1/5] Build tom-relay"
cargo build --release -p tom-relay

if [[ "${AIR_COPY_STRESS}" == "1" ]]; then
  echo "[1b/5] Build tom-stress"
  cargo build --release -p tom-stress
fi

echo "[2/5] Create USB bundle at ${USB_OUT_DIR}"
mkdir -p "${USB_OUT_DIR}/bin"

echo "[3/5] Copy binaries"
cp -f target/release/tom-relay "${USB_OUT_DIR}/bin/tom-relay"
chmod +x "${USB_OUT_DIR}/bin/tom-relay"

if [[ "${AIR_COPY_STRESS}" == "1" ]]; then
  cp -f target/release/tom-stress "${USB_OUT_DIR}/bin/tom-stress"
  chmod +x "${USB_OUT_DIR}/bin/tom-stress"
fi

echo "[4/5] Generate launchd plist template"
sed \
  -e "s|REPLACE_USER|${AIR_USER}|g" \
  -e "s|3341|${AIR_RELAY_PORT}|g" \
  -e "s|9091|${AIR_METRICS_PORT}|g" \
  "deploy/macos/com.tom.relay.air.plist" > "${USB_OUT_DIR}/com.tom.relay.air.plist"

echo "[5/5] Install helper script + README"
if [[ "$(cd "${USB_OUT_DIR}" && pwd)" == "$(cd "${TEMPLATE_DIR}" && pwd)" ]]; then
  echo "ERROR: USB_OUT_DIR must be different from ${TEMPLATE_DIR}" >&2
  echo "Example: USB_OUT_DIR=target/usb-mac-air-relay-bundle ./scripts/prepare-usb-mac-air-relay.sh" >&2
  exit 1
fi

cp -f "${TEMPLATE_DIR}/install-on-air.sh" "${USB_OUT_DIR}/install-on-air.sh"
cp -f "${TEMPLATE_DIR}/README.md" "${USB_OUT_DIR}/README.md"
chmod +x "${USB_OUT_DIR}/install-on-air.sh"

cat <<EOF

USB bundle ready: ${USB_OUT_DIR}

Next:
1) Copy this folder to USB key
2) On MacBook Air, open Terminal in that folder
3) Run: ./install-on-air.sh

EOF
