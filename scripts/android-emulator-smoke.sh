#!/usr/bin/env bash
# Build bichon for Android, boot an emulator AVD, adb push, run briefly, and verify
# the process stays up (catches immediate crashes).
#
# On an **x86_64 Linux** host (typical EC2), the Android Emulator only runs **x86_64**
# system images. You cannot boot **arm64-v8a** guests there (QEMU2 rejects it). For
# that case this script builds **x86_64-linux-android** — still full Bionic/NDK — which
# is the right smoke for "runs on Android". Ship **aarch64-linux-android** to devices.
#
# Environment:
#   ANDROID_SDK_ROOT or ANDROID_HOME
#   ANDROID_NDK_HOME
#   ANDROID_AVD_NAME   (default: Cory36 if listed by avdmanager, else bichon_smoke_api36_x86)
#   ANDROID_API_LEVEL  (default 36; must match your NDK clang suffix)
#   ANDROID_SMOKE_SECONDS, ANDROID_EMULATOR_BOOT_WAIT, ANDROID_EMULATOR_GPU, ANDROID_EMULATOR_QEMU_CPU
#
# Usage:
#   export ANDROID_SDK_ROOT=.../Android/Sdk ANDROID_NDK_HOME=.../ndk/30.x
#   ./scripts/android-emulator-smoke.sh

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

API_LEVEL="${ANDROID_API_LEVEL:-36}"
REMOTE_BIN="/data/local/tmp/bichon_smoke"
REMOTE_ROOT="/data/local/tmp/bichon_smoke_root"
RUN_SECONDS="${ANDROID_SMOKE_SECONDS:-12}"
HTTP_PORT="${ANDROID_SMOKE_HTTP_PORT:-18080}"
GPU_MODE="${ANDROID_EMULATOR_GPU:-guest}"
BOOT_WAIT_ROUNDS="${ANDROID_EMULATOR_BOOT_WAIT:-200}"
QEMU_CPU="${ANDROID_EMULATOR_QEMU_CPU:-qemu64}"

# x86_64 guest + Rust triple (only combination that runs on x86_64 Linux emulator hosts)
RUST_TARGET="x86_64-linux-android"
PACKAGE="system-images;android-36.1;google_apis;x86_64"
DEFAULT_FALLBACK_AVD="bichon_smoke_api36_x86"

if [[ -z "${ANDROID_NDK_HOME:-}" ]]; then
  echo "error: ANDROID_NDK_HOME is not set" >&2
  exit 1
fi

SDK="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-}}"
if [[ -z "$SDK" ]]; then
  echo "error: set ANDROID_SDK_ROOT or ANDROID_HOME" >&2
  exit 1
fi

export ANDROID_SDK_ROOT="$SDK"
ADB_BASE="$SDK/platform-tools/adb"
EMULATOR="$SDK/emulator/emulator"
AVDMANAGER="$SDK/cmdline-tools/latest/bin/avdmanager"

for x in "$ADB_BASE" "$EMULATOR" "$AVDMANAGER"; do
  if [[ ! -x "$x" ]]; then
    echo "error: missing or not executable: $x" >&2
    exit 1
  fi
done

resolve_avd_name() {
  if [[ -n "${ANDROID_AVD_NAME:-}" ]]; then
    echo "$ANDROID_AVD_NAME"
    return
  fi
  if "$AVDMANAGER" list avd 2>/dev/null | grep -q "Name: Cory36"; then
    echo "Cory36"
    return
  fi
  echo "$DEFAULT_FALLBACK_AVD"
}

AVD_NAME="$(resolve_avd_name)"

ADB_SERIAL=""

pick_emulator_serial() {
  ADB_SERIAL=$("$ADB_BASE" devices | awk '/^emulator-/ {print $1; exit}')
  if [[ -z "${ADB_SERIAL:-}" ]]; then
    return 1
  fi
  echo "Using adb serial: $ADB_SERIAL"
}

ADB() { "$ADB_BASE" -s "$ADB_SERIAL" "$@"; }

reset_adb() {
  "$ADB_BASE" kill-server 2>/dev/null || true
  sleep 1
  "$ADB_BASE" start-server
}

if ! rustup target list --installed | grep -qx "$RUST_TARGET"; then
  rustup target add "$RUST_TARGET"
fi

echo "=== Building $RUST_TARGET (release) for emulator smoke ==="
ANDROID_RUST_TARGET="$RUST_TARGET" \
  ANDROID_NDK_HOME="$ANDROID_NDK_HOME" \
  ANDROID_API_LEVEL="$API_LEVEL" \
  ./scripts/android-build.sh --release

LOCAL_BIN="$ROOT/target/$RUST_TARGET/release/bichon"
if [[ ! -x "$LOCAL_BIN" ]]; then
  echo "error: binary not found: $LOCAL_BIN" >&2
  exit 1
fi

echo "=== Ensuring AVD exists: $AVD_NAME ==="
if ! "$AVDMANAGER" list avd 2>/dev/null | grep -q "Name: $AVD_NAME"; then
  echo no | "$AVDMANAGER" create avd -n "$AVD_NAME" -k "$PACKAGE" -d pixel_7
fi

reset_adb
"$ADB_BASE" emu kill 2>/dev/null || true
sleep 2
pkill -f "qemu-system-x86_64" 2>/dev/null || true
sleep 2

echo "=== Starting emulator (AVD=$AVD_NAME, no KVM, gpu=$GPU_MODE, -cpu $QEMU_CPU) ==="
"$EMULATOR" -avd "$AVD_NAME" -no-window -no-audio -no-snapshot \
  -gpu "$GPU_MODE" -accel off -no-boot-anim \
  -netdelay none -netspeed full \
  -qemu -cpu "$QEMU_CPU" \
  >/tmp/bichon_emulator.log 2>&1 &
EMU_PID=$!

cleanup() {
  kill "$EMU_PID" 2>/dev/null || true
  "$ADB_BASE" emu kill 2>/dev/null || true
}
trap cleanup EXIT

echo "=== Waiting for adb ==="
for _ in $(seq 1 180); do
  if pick_emulator_serial 2>/dev/null; then
    break
  fi
  if "$ADB_BASE" devices 2>/dev/null | grep -q offline; then
    "$ADB_BASE" reconnect 2>/dev/null || true
  fi
  sleep 2
done
if ! pick_emulator_serial 2>/dev/null; then
  echo "error: no emulator appeared in adb devices" >&2
  "$ADB_BASE" devices >&2
  tail -80 /tmp/bichon_emulator.log >&2 || true
  exit 1
fi

ADB wait-for-device
for _ in $(seq 1 300); do
  state=$(ADB get-state 2>/dev/null | tr -d '\r' || true)
  if [[ "$state" == "device" ]]; then
    break
  fi
  "$ADB_BASE" reconnect 2>/dev/null || true
  sleep 2
done
state=$(ADB get-state 2>/dev/null | tr -d '\r' || true)
if [[ "$state" != "device" ]]; then
  echo "error: adb never reached state=device for $ADB_SERIAL (got: ${state:-unknown})" >&2
  tail -80 /tmp/bichon_emulator.log >&2 || true
  exit 1
fi

BOOT=""
for _ in $(seq 1 "$BOOT_WAIT_ROUNDS"); do
  BOOT=$(ADB shell getprop sys.boot_completed 2>/dev/null | tr -d '\r' || true)
  if [[ "$BOOT" == "1" ]]; then
    break
  fi
  sleep 3
done
if [[ "$BOOT" != "1" ]]; then
  echo "error: sys.boot_completed != 1" >&2
  tail -80 /tmp/bichon_emulator.log >&2 || true
  exit 1
fi

echo "=== Pushing binary ==="
ADB push "$LOCAL_BIN" "$REMOTE_BIN"
ADB shell "rm -rf $REMOTE_ROOT && mkdir -p $REMOTE_ROOT && chmod 755 $REMOTE_BIN"

echo "=== Running bichon for ${RUN_SECONDS}s (HTTP :$HTTP_PORT) ==="
ADB shell "nohup $REMOTE_BIN \
  --bichon-root-dir $REMOTE_ROOT \
  --bichon-http-port $HTTP_PORT \
  --bichon-public-url http://127.0.0.1:$HTTP_PORT \
  --bichon-bind-ip 127.0.0.1 \
  </dev/null >/data/local/tmp/bichon_smoke.out 2>&1 & echo \$! > /data/local/tmp/bichon_smoke.pid"

sleep "$RUN_SECONDS"

PID=$(ADB shell "cat /data/local/tmp/bichon_smoke.pid 2>/dev/null | tr -d '\r'" || true)
if [[ -z "${PID:-}" ]]; then
  echo "error: could not read PID file" >&2
  ADB shell "cat /data/local/tmp/bichon_smoke.out 2>/dev/null || true" >&2
  exit 1
fi

if ! ADB shell "test -d /proc/$PID" 2>/dev/null; then
  echo "error: bichon PID $PID not running (early exit / crash)" >&2
  ADB shell "cat /data/local/tmp/bichon_smoke.out 2>/dev/null || true" >&2
  exit 1
fi

echo "=== Process $PID still running after ${RUN_SECONDS}s ==="
if ADB shell "command -v curl >/dev/null 2>&1"; then
  ADB shell "curl -s -o /dev/null -w '%{http_code}' http://127.0.0.1:$HTTP_PORT/api/status || true" || true
  echo
fi

ADB shell "kill $PID" 2>/dev/null || true
echo "=== Smoke test OK (x86_64 Android guest; use aarch64-linux-android on ARM hardware) ==="
