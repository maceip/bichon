# Android (NDK) build notes

The server binary is intended to run on-device with a persistent data directory (same layout as desktop) so **stop/start does not lose data**: indexes and blobs stay on disk under `bichon_root_dir`.

## Recommended feature set

- **`--no-default-features`** — disables `mimalloc` (not used on Android), `cli-tools` (desktop import CLIs), and `system-stats` (optional `sysinfo`).
- Android app clients talk to the **HTTP API** only; no JNI or Rust bindings are required for that path.

## Build

1. Install Rust and the Android target:

   ```bash
   rustup target add aarch64-linux-android
   ```

2. Install the [Android NDK](https://developer.android.com/ndk) and set `ANDROID_NDK_HOME`.

3. From the repo root:

   ```bash
   chmod +x scripts/android-build.sh
   ANDROID_NDK_HOME=/path/to/ndk ANDROID_API_LEVEL=36 ./scripts/android-build.sh --release
   ```

   If `android-36` is not present in your NDK, use an installed API level (for example `35`) via `ANDROID_API_LEVEL`.

4. **Web UI**: before release builds that should ship the real UI, run `pnpm install && pnpm run build` in `web/`. If `web/dist` is missing, `build.rs` creates a minimal stub so `cargo check` still works.

## Process lifecycle

Graceful shutdown is triggered via the same internal signal path as desktop (`SIGNAL_MANAGER` + Poem graceful shutdown). On Android, **SIGTERM** is not wired; prefer **Ctrl+C** in a shell or terminating the process from your wrapper so in-flight work can flush (Tantivy commit, blob queue, etc.).
