#!/usr/bin/env bash
# Build the Bichon server for Android (aarch64) using the NDK LLVM toolchain.
# Targets API 36 by default (override with ANDROID_API_LEVEL).
#
# Prerequisites:
#   - Rust target: rustup target add aarch64-linux-android
#   - Android NDK (r26+): set ANDROID_NDK_HOME to the NDK root
#
# The Android build omits desktop CLI tools and mimalloc; use the same flags in your app wrapper.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

API_LEVEL="${ANDROID_API_LEVEL:-36}"
TARGET="aarch64-linux-android"

if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  echo "error: ANDROID_NDK_HOME is not set (path to Android NDK)" >&2
  exit 1
fi

NDK="$ANDROID_NDK_HOME"
case "$(uname -s)" in
  Darwin)
    PREBUILT="$NDK/toolchains/llvm/prebuilt/darwin-x86_64"
    ;;
  Linux)
    PREBUILT="$NDK/toolchains/llvm/prebuilt/linux-x86_64"
    ;;
  *)
    echo "error: unsupported host OS for NDK prebuilt" >&2
    exit 1
    ;;
esac

if [[ ! -d "$PREBUILT" ]]; then
  echo "error: NDK prebuilt not found at $PREBUILT" >&2
  exit 1
fi

CC_BIN="$PREBUILT/bin/${TARGET}${API_LEVEL}-clang"
CXX_BIN="$PREBUILT/bin/${TARGET}${API_LEVEL}-clang++"
AR_BIN="$PREBUILT/bin/llvm-ar"

if [[ ! -x "$CC_BIN" ]]; then
  echo "error: clang not found: $CC_BIN" >&2
  echo "hint: install an NDK that provides android-${API_LEVEL}, or set ANDROID_API_LEVEL to an installed API (e.g. 35)." >&2
  exit 1
fi

export CC_${TARGET//-/_}="$CC_BIN"
export CXX_${TARGET//-/_}="$CXX_BIN"
export AR_${TARGET//-/_}="$AR_BIN"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_BIN"
export CARGO_TARGET_AARCH64_LINUX_ANDROID_AR="$AR_BIN"

# ring / aws-lc-sys and other -sys crates
export BINDGEN_EXTRA_CLANG_ARGS="--sysroot=$PREBUILT/sysroot"

echo "Using NDK clang: $CC_BIN (API $API_LEVEL)"
exec cargo build --target "$TARGET" --no-default-features "$@"
