// Locating and reporting the SBF build toolchain (cargo-build-sbf).
//
// PexliFormation drives the Pexli/Solana-style SBF compiler. That compiler is
// shipped as `cargo-build-sbf` (part of the Solana/Agave platform-tools). We
// look for it in three places, in order:
//   1. A copy bundled inside the app (resources/platform-tools) — this is what
//      makes the app fully offline.
//   2. The PATH (a developer machine that already has Solana/Agave installed).
//   3. The default per-user install location (~/.local/share/solana ...).

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::Serialize;
use tauri::Manager;

#[cfg(windows)]
const EXE: &str = "cargo-build-sbf.exe";
#[cfg(not(windows))]
const EXE: &str = "cargo-build-sbf";

#[derive(Serialize, Clone)]
pub struct ToolchainInfo {
    pub available: bool,
    pub path: Option<String>,
    pub version: Option<String>,
}

/// Resolve the full path to `cargo-build-sbf`, or `None` if not installed.
pub fn resolve(app: &tauri::AppHandle) -> Option<PathBuf> {
    // 1. Bundled with the app (offline path).
    if let Ok(res_dir) = app.path().resource_dir() {
        let bundled = res_dir
            .join("resources")
            .join("platform-tools")
            .join("bin")
            .join(EXE);
        if bundled.is_file() {
            return Some(bundled);
        }
    }

    // 2. On PATH.
    if let Ok(p) = which::which("cargo-build-sbf") {
        return Some(p);
    }

    // 3. Common per-user install locations.
    for base in default_install_roots() {
        let candidate = base.join("bin").join(EXE);
        if candidate.is_file() {
            return Some(candidate);
        }
    }

    None
}

fn default_install_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = home_dir() {
        // Agave / Solana active_release layout.
        roots.push(home.join(".local").join("share").join("solana").join("install").join("active_release"));
    }
    roots
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .map(PathBuf::from)
}

/// Report toolchain availability + version to the UI.
pub fn info(app: &tauri::AppHandle) -> ToolchainInfo {
    match resolve(app) {
        Some(path) => {
            let version = Command::new(&path)
                .arg("--version")
                .output()
                .ok()
                .and_then(|o| {
                    let s = String::from_utf8_lossy(&o.stdout);
                    s.lines().next().map(|l| l.trim().to_string())
                });
            ToolchainInfo {
                available: true,
                path: Some(path.display().to_string()),
                version,
            }
        }
        None => ToolchainInfo {
            available: false,
            path: None,
            version: None,
        },
    }
}

/// First-run install: `cargo-build-sbf` self-downloads the platform-tools the
/// first time it is invoked. If it is not yet on the system at all we fall back
/// to the bundled copy check. This step needs the internet exactly once; after
/// that the toolchain lives on disk and the app runs fully offline.
pub fn install(app: &tauri::AppHandle) -> Result<(), String> {
    if resolve(app).is_some() {
        // Trigger platform-tools download by asking for the version.
        if let Some(path) = resolve(app) {
            let _ = Command::new(path).arg("--version").output();
        }
        return Ok(());
    }
    Err(
        "cargo-build-sbf was not found. Install the Solana/Agave CLI once, or \
         reinstall PexliFormation with the bundled toolchain, then reopen the app."
            .into(),
    )
}

/// Directory that should be on PATH so `cargo-build-sbf` can find its sibling
/// tools (rustc/llvm from platform-tools).
pub fn bin_dir(tool: &Path) -> Option<PathBuf> {
    tool.parent().map(|p| p.to_path_buf())
}
