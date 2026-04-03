#!/bin/bash

# Add Rust to PATH - Xcode runs scripts in a clean environment
export PATH="$HOME/.cargo/bin:/usr/local/bin:$PATH"

# Ne pas arrêter sur les erreurs pour afficher plus d'infos
echo "========================================="
echo "🔨 Build tom_protocol_ffi pour tvOS"
echo "========================================="

# Afficher les variables d'environnement Xcode
echo ""
echo "📋 Variables d'environnement:"
echo "  EFFECTIVE_PLATFORM_NAME: ${EFFECTIVE_PLATFORM_NAME:-<non défini>}"
echo "  PLATFORM_NAME: ${PLATFORM_NAME:-<non défini>}"
echo "  SRCROOT: ${SRCROOT:-<non défini>}"
echo "  PROJECT_DIR: ${PROJECT_DIR:-<non défini>}"

# Déterminer la cible (iOS ou tvOS)
PLATFORM="${EFFECTIVE_PLATFORM_NAME:-${PLATFORM_NAME:-}}"
case "$PLATFORM" in
    -iphoneos|iphoneos)
        TVOS_TARGET="aarch64-apple-ios"
        ;;
    -iphonesimulator|iphonesimulator)
        TVOS_TARGET="aarch64-apple-ios-sim"
        ;;
    -appletvsimulator|appletvsimulator)
        TVOS_TARGET="aarch64-apple-tvos-sim"
        ;;
    -appletvos|appletvos|*)
        TVOS_TARGET="aarch64-apple-tvos"
        ;;
esac

echo "  Target Rust: $TVOS_TARGET"

# Trouver les répertoires
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
echo ""
echo "📁 Répertoires:"
echo "  Script: $SCRIPT_DIR"

# Essayer de trouver le workspace Cargo.toml (racine du projet Rust)
SEARCH_DIRS=(
    "$SCRIPT_DIR/../.."
    "$SCRIPT_DIR/../../.."
    "$SCRIPT_DIR/../../../.."
    "/Users/malik/Documents/tom-protocol"
    "${SRCROOT:-}/../.."
    "${PROJECT_DIR:-}/../.."
)

WORKSPACE_ROOT=""
for dir in "${SEARCH_DIRS[@]}"; do
    if [ -d "$dir" ] && [ -f "$dir/Cargo.toml" ]; then
        # Vérifier que c'est un workspace avec tom-protocol-ffi
        if grep -q "tom-protocol-ffi" "$dir/Cargo.toml" 2>/dev/null; then
            WORKSPACE_ROOT="$(cd "$dir" && pwd)"
            echo "  ✅ Workspace trouvé: $WORKSPACE_ROOT"
            break
        fi
    fi
done

if [ -z "$WORKSPACE_ROOT" ]; then
    echo ""
    echo "❌ ERREUR: Impossible de trouver le workspace Cargo avec tom-protocol-ffi"
    echo ""
    echo "Chemins recherchés:"
    for dir in "${SEARCH_DIRS[@]}"; do
        echo "  - $dir"
    done
    echo ""
    echo "💡 SOLUTION: Vérifiez que ce script est dans le bon répertoire"
    echo "   ou modifiez manuellement la variable FFI_DIR ci-dessous:"
    echo ""
    echo "   FFI_DIR=\"/chemin/vers/tom-protocol-ffi\""
    echo ""
    exit 1
fi

# Trouver le crate FFI
FFI_DIR="$WORKSPACE_ROOT/crates/tom-protocol-ffi"
if [ ! -d "$FFI_DIR" ]; then
    FFI_DIR="$WORKSPACE_ROOT/tom-protocol-ffi"
fi

if [ ! -f "$FFI_DIR/Cargo.toml" ]; then
    echo "❌ ERREUR: Cargo.toml introuvable dans: $FFI_DIR"
    exit 1
fi

echo "  FFI crate: $FFI_DIR"

# Vérifier Rust
echo ""
echo "🦀 Vérification de Rust..."
if ! command -v cargo &> /dev/null; then
    echo "❌ ERREUR: Cargo n'est pas installé"
    echo "   Installez depuis: https://rustup.rs/"
    exit 1
fi

CARGO_VERSION=$(cargo --version)
echo "  ✅ $CARGO_VERSION"

# Installer la toolchain nightly (nécessaire pour tvOS)
echo "  ⚙️  Installation de la toolchain nightly..."
rustup toolchain install nightly 2>/dev/null || true

# Vérifier la cible pour nightly
if ! rustup target list --toolchain nightly --installed | grep -q "$TVOS_TARGET"; then
    echo "  ⚠️  Installation de la cible $TVOS_TARGET pour nightly..."
    rustup target add --toolchain nightly "$TVOS_TARGET" || {
        echo "❌ ERREUR: Impossible d'installer la cible $TVOS_TARGET"
        echo ""
        echo "💡 Essayez manuellement:"
        echo "   rustup toolchain install nightly"
        echo "   rustup target add --toolchain nightly $TVOS_TARGET"
        exit 1
    }
fi

echo "  ✅ Cible $TVOS_TARGET installée"

# Compiler avec nightly
echo ""
echo "🔧 Compilation..."
cd "$FFI_DIR" || exit 1

cargo +nightly build --release --target "$TVOS_TARGET" 2>&1 || {
    echo ""
    echo "❌ ERREUR: La compilation Rust a échoué"
    echo ""
    echo "💡 Si l'erreur persiste, essayez:"
    echo "   cd $FFI_DIR"
    echo "   cargo +nightly build --release --target $TVOS_TARGET -vv"
    exit 1
}

# Vérifier la bibliothèque
LIB_PATH="$FFI_DIR/target/$TVOS_TARGET/release/libtom_protocol_ffi.a"
if [ ! -f "$LIB_PATH" ]; then
    echo "❌ ERREUR: Bibliothèque non créée: $LIB_PATH"
    exit 1
fi

echo "  ✅ Compilation réussie"

# Copier
BUILD_DIR="$SCRIPT_DIR/build"
mkdir -p "$BUILD_DIR"

cp "$LIB_PATH" "$BUILD_DIR/libtom_protocol_ffi.a"

echo ""
echo "📦 Bibliothèque copiée vers:"
echo "   $BUILD_DIR/libtom_protocol_ffi.a"

# Vérifier l'architecture
echo ""
echo "🔍 Architecture:"
lipo -info "$BUILD_DIR/libtom_protocol_ffi.a"

echo ""
echo "✅ Build terminé avec succès!"
echo "========================================="
