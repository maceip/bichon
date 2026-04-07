#!/usr/bin/env bash
# Build the Bichon server for Android using the NDK LLVM toolchain.
# Targets API 36 by default (override with ANDROID_API_LEVEL).
#
# Set ANDROID_RUST_TARGET to choose the Rust/LLVM triple:
#   aarch64-linux-android  (default, physical arm64 devices)
#   x86_64-linux-android   (emulator on x86_64 hosts without ARM translation)
#
# Prerequisites:
#   - rustup target add "$ANDROID_RUST_TARGET"
#   - Android NDK (r26+): ANDROID_NDK_HOME
#
# The Android build omits desktop CLI tools and mimalloc; use the same flags in your app wrapper.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

API_LEVEL="${ANDROID_API_LEVEL:-36}"
TARGET="${ANDROID_RUST_TARGET:-aarch64-linux-android}"

case "$TARGET" in
  aarch64-linux-android | x86_64-linux-android) ;;
  *)
    echo "error: unsupported ANDROID_RUST_TARGET='$TARGET' (use aarch64-linux-android or x86_64-linux-android)" >&2
    exit 1
    ;;
esac

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

# cc-rs / cargo: map TARGET triple to env var prefix (hyphens -> underscores)
VAR_PREFIX="${TARGET//-/_}"

export CC_${VAR_PREFIX}="$CC_BIN"
export CXX_${VAR_PREFIX}="$CXX_BIN"
export AR_${VAR_PREFIX}="$AR_BIN"

case "$TARGET" in
  aarch64-linux-android)
    export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$CC_BIN"
    export CARGO_TARGET_AARCH64_LINUX_ANDROID_AR="$AR_BIN"
    ;;
  x86_64-linux-android)
    export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$CC_BIN"
    export CARGO_TARGET_X86_64_LINUX_ANDROID_AR="$AR_BIN"
    ;;
esac

# ring / aws-lc-sys and other -sys crates
export BINDGEN_EXTRA_CLANG_ARGS="--sysroot=$PREBUILT/sysroot"

echo "Using NDK clang: $CC_BIN (API $API_LEVEL, target $TARGET)"
exec cargo build --target "$TARGET" --no-default-features "$@"
