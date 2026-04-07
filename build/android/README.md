# Android (NDK) build notes

The server binary is intended to run on-device with a persistent data directory (same layout as desktop) so **stop/start does not lose data**: indexes and blobs stay on disk under `bichon_root_dir`.

## Recommended feature set

- **`--no-default-features`** — disables `mimalloc` (not used on Android), `cli-tools` (desktop import CLIs), and `system-stats` (optional `sysinfo`).
- Android app clients talk to the **HTTP API** only; no JNI or Rust bindings are required for that path.

## Build

1. Install Rust and the Android target(s):

   ```bash
   rustup target add aarch64-linux-android
   # Optional: for x86_64 emulator on an x86_64 host (no ARM translation)
   rustup target add x86_64-linux-android
   ```

2. Install the [Android NDK](https://developer.android.com/ndk) and set `ANDROID_NDK_HOME`.

3. From the repo root:

   ```bash
   chmod +x scripts/android-build.sh
   ANDROID_NDK_HOME=/path/to/ndk ANDROID_API_LEVEL=36 ./scripts/android-build.sh --release
   ```

   If `android-36` is not present in your NDK, use an installed API level (for example `35`) via `ANDROID_API_LEVEL`.

### Verified toolchain

A full **`--release`** build of `bichon` for **`aarch64-linux-android`** completed successfully with:

- Android NDK **30.0.14904198** (`aarch64-linux-android36-clang`)
- Rust **1.94.1** stable, `cargo build --target aarch64-linux-android --no-default-features --release`

Output binary: `target/aarch64-linux-android/release/bichon`.

**x86_64 emulator (x86_64 Linux host, no KVM):** set `ANDROID_RUST_TARGET=x86_64-linux-android` when calling `scripts/android-build.sh`. The artifact is `target/x86_64-linux-android/release/bichon`. Use this to run the server inside an API 36 **x86_64** AVD on a typical cloud VM.

**One-shot emulator smoke test** (build **x86_64-linux-android**, boot an **x86_64** API 36 AVD, `adb push`, run ~12s, fail if the process dies):

```bash
export ANDROID_SDK_ROOT="$HOME/Android/Sdk"
export ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/30.0.14904198"
# Uses AVD `Cory36` when present, else creates `bichon_smoke_api36_x86`.
./scripts/android-emulator-smoke.sh
```

On **x86_64 Linux** hosts (including typical EC2), the emulator **cannot** run **arm64-v8a** system images; smoke-test the **same Bionic stack** via **x86_64-linux-android**, and ship **aarch64-linux-android** to **physical ARM** devices.

To run the same flow on a remote host that already has SDK/NDK installed, sync the tree (omit `.git` and `target`), then:

```bash
cd ~/bichon-android-build
export ANDROID_NDK_HOME="$HOME/Android/Sdk/ndk/30.0.14904198"
export ANDROID_API_LEVEL=36
./scripts/android-build.sh --release
```

4. **Web UI**: before release builds that should ship the real UI, run `pnpm install && pnpm run build` in `web/`. If `web/dist` is missing, `build.rs` creates a minimal stub so `cargo check` still works.

## Process lifecycle

Graceful shutdown is triggered via the same internal signal path as desktop (`SIGNAL_MANAGER` + Poem graceful shutdown). On Android, **SIGTERM** is not wired; prefer **Ctrl+C** in a shell or terminating the process from your wrapper so in-flight work can flush (Tantivy commit, blob queue, etc.).

## Device / emulator verification

- **x86_64 Linux + no physical device**: `scripts/android-emulator-smoke.sh` is meant to boot an **x86_64** API 36 guest (software rendering, `-accel off`, `-cpu qemu64`) and run the **x86_64-linux-android** binary. On **KVM-less** cloud VMs this is **best-effort**: we have seen `adb` stuck **`offline`**, QEMU **segfaults** with some `-gpu` modes, and **`adb protocol fault`** when the emulator drops mid-boot. If the script fails, use a **physical device**, a **KVM-capable** machine, or **AWS Device Farm** with a wrapper that installs your binary.
- **Physical ARM64 device**: push **`aarch64-linux-android/release/bichon`** and run with the same CLI flags. The Android Emulator on an **x86_64** host **cannot** run **arm64-v8a** system images (QEMU2 rejects `abi.type=arm64` on `x86_64` hosts), so there is no arm64-guest substitute on that hardware.

### What was verified on the shared EC2 builder

- **`aarch64-linux-android` release** `bichon` links and the ELF is valid (API **36** NDK **30.0.14904198**).
- **`x86_64-linux-android` release** builds cleanly (same NDK) for emulator smoke targets.
- **End-to-end emulator + adb run** did not complete reliably on that instance without KVM; treat **on-device** execution as the acceptance check for PRs that require “ran once on Android.”
- **`qemu-user-static` + NDK sysroot alone** does **not** provide `/system/bin/linker64`; it cannot run the PIE the NDK linker produces.
- **AWS Device Farm**: an account **project** exists (`Cory` in `us-west-2`), but this repository ships a **native executable**, not an APK. Running it on Device Farm would need a **small wrapper app** or **custom test spec** that installs and launches the binary (out of scope for the compile-only harness).

**Unit tests (`cargo test --lib`)** can be run on any Linux host with a normal Rust toolchain; several tests in-tree still **fail in a clean checkout** because they reference **developer-local paths** (Windows `C:\...`, `e:\test.mbox`), **live IMAP credentials**, or an **SMTP server on localhost**—they are integration-style checks, not hermetic CI tests.
