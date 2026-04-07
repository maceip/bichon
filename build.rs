use std::{
    fs,
    io::Result,
    path::PathBuf,
    process::Command,
};

fn ensure_frontend_dist_stub() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let dist = manifest_dir.join("web/dist");
    let index = dist.join("index.html");
    if index.exists() {
        return;
    }
    let _ = fs::create_dir_all(&dist);
    let stub = concat!(
        "<!DOCTYPE html><html><head><meta charset=\"utf-8\"/>",
        "<title>Bichon</title></head><body>",
        "<p>Web UI assets are missing. Run <code>pnpm install && pnpm run build</code> in <code>web/</code>, ",
        "then rebuild the server.</p></body></html>",
    );
    let _ = fs::write(&index, stub);
}

fn main() -> Result<()> {
    ensure_frontend_dist_stub();
    if cfg!(target_os = "windows") {
        println!("cargo:rustc-link-lib=Rstrtmgr");
    }
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string());
    println!("cargo:rustc-env=GIT_HASH={}", git_hash);
    Ok(())
}
