#!/usr/bin/env bash
set -euo pipefail

TVOS_TOOLCHAIN="${TVOS_TOOLCHAIN:-nightly-aarch64-apple-darwin}"

echo "== Apple TV preflight =="

if ! command -v xcodebuild >/dev/null 2>&1; then
  echo "Missing: xcodebuild (install Xcode first)" >&2
  exit 1
fi

if ! command -v rustup >/dev/null 2>&1; then
  echo "Missing: rustup" >&2
  exit 1
fi

if ! command -v cargo >/dev/null 2>&1; then
  echo "Missing: cargo" >&2
  exit 1
fi

echo "Xcode version:"
xcodebuild -version

echo "Installed tvOS SDKs:"
xcodebuild -showsdks | grep -i tvos || true

echo "Ensuring Rust tvOS target is installed..."
rustup toolchain install "${TVOS_TOOLCHAIN}" >/dev/null
rustup target add aarch64-apple-tvos --toolchain "${TVOS_TOOLCHAIN}" >/dev/null

echo "Rust target installed:"
rustup target list --installed --toolchain "${TVOS_TOOLCHAIN}" | grep aarch64-apple-tvos

echo "Toolchain used: ${TVOS_TOOLCHAIN}"

echo "Preflight OK"
