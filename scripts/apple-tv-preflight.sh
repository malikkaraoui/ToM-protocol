#!/usr/bin/env bash
set -euo pipefail

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
rustup target add aarch64-apple-tvos >/dev/null

echo "Rust target installed:"
rustup target list --installed | grep aarch64-apple-tvos

echo "Preflight OK"
