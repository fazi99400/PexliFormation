// Locating AND automatically setting up the SBF build toolchain
// (cargo-build-sbf).
//
// PexliFormation drives the Pexli/Solana-style SBF compiler. That compiler is
// shipped as `cargo-build-sbf` (part of the Solana/Agave platform-tools). We
// look for it in three places, in order:
//   1. A copy bundled inside the app (resources/platform-tools) — fully offline.
//   2. The PATH (a machine that already has Solana/Agave installed).
//   3. The default per-user install location (~/.local/share/solana ...).
//
// When it is missing, `install()` performs the whole one-time setup itself
// (Rust + Agave + platform-tools) so the user only has to click once — no
// terminal, no manual steps.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;
use tauri::{Emitter, Manager};

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
        roots.push(
            home.join(".local")
                .join("share")
                .join("solana")
                .join("install")
                .join("active_release"),
        );
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

/// One-click setup. If the toolchain is already present we just warm up the
/// platform-tools; otherwise we install everything automatically and stream the
/// progress to the UI. This needs the internet once; afterwards the app runs
/// fully offline.
pub fn install(app: &tauri::AppHandle) -> Result<(), String> {
    if let Some(tool) = resolve(app) {
        emit(app, "SBF toolchain found — fetching the compiler (platform-tools) if needed…");
        let _ = warmup(app, &tool);
        return Ok(());
    }

    #[cfg(windows)]
    {
        install_windows(app)?;
        match resolve(app) {
            Some(tool) => {
                let _ = warmup(app, &tool);
                emit(app, "Setup complete. You can now convert contracts.");
                Ok(())
            }
            None => Err(
                "Setup ran but cargo-build-sbf was still not found. Please close and reopen \
                 PexliFormation, then try again."
                    .into(),
            ),
        }
    }

    #[cfg(not(windows))]
    {
        Err("Automatic setup is currently implemented for Windows only.".into())
    }
}

/// Run the bundled PowerShell setup (Rust + Agave + platform-tools), streaming
/// every line to the UI so the user sees live progress.
#[cfg(windows)]
fn install_windows(app: &tauri::AppHandle) -> Result<(), String> {
    // Single source of truth: the same script shipped in scripts/.
    const SCRIPT: &str = include_str!("../../scripts/setup-toolchain.ps1");

    let path = std::env::temp_dir().join("pexliformation-setup.ps1");
    std::fs::write(&path, SCRIPT).map_err(|e| format!("Cannot write setup script: {e}"))?;

    emit(app, "Starting one-time SBF toolchain setup…");
    emit(app, "This downloads the Rust + SBF compiler once (a few hundred MB). Please wait.");

    let mut cmd = Command::new("powershell");
    cmd.args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
        .arg(&path);
    stream(app, cmd)
}

/// Ask cargo-build-sbf for its version, which triggers the first-time
/// platform-tools download if it has not happened yet.
fn warmup(app: &tauri::AppHandle, tool: &Path) -> Result<(), String> {
    let mut cmd = Command::new(tool);
    cmd.arg("--version");
    stream(app, cmd)
}

/// Spawn a command and stream stdout + stderr to the UI without deadlocking.
fn stream(app: &tauri::AppHandle, mut cmd: Command) -> Result<(), String> {
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start setup process: {e}"))?;

    // Drain stderr on a separate thread so a full pipe can't block stdout.
    let stderr = child.stderr.take();
    let app2 = app.clone();
    let joiner = std::thread::spawn(move || {
        if let Some(e) = stderr {
            for line in BufReader::new(e).lines().map_while(Result::ok) {
                let _ = app2.emit("build-log", line);
            }
        }
    });

    if let Some(out) = child.stdout.take() {
        for line in BufReader::new(out).lines().map_while(Result::ok) {
            emit(app, &line);
        }
    }

    let _ = joiner.join();
    let status = child.wait().map_err(|e| e.to_string())?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "Setup exited with code {} — see the log above.",
            status.code().unwrap_or(-1)
        ))
    }
}

/// Directory that should be on PATH so `cargo-build-sbf` can find its sibling
/// tools (rustc/llvm from platform-tools).
pub fn bin_dir(tool: &Path) -> Option<PathBuf> {
    tool.parent().map(|p| p.to_path_buf())
}

#[cfg(windows)]
const HOST_CARGO: &str = "cargo.exe";
#[cfg(not(windows))]
const HOST_CARGO: &str = "cargo";

/// The bundled host toolchain bin dir (the `rust/bin` inside platform-tools
/// that holds `cargo`/`rustc`). `cargo-build-sbf` shells out to `cargo` for
/// metadata, so this must be on PATH — and we want *our* bundled one, never the
/// user's system Rust. Returns `None` if we're not running from a bundle.
pub fn bundled_host_bin(app: &tauri::AppHandle) -> Option<PathBuf> {
    let res = app.path().resource_dir().ok()?;
    let root = res.join("resources").join("platform-tools");
    // The usual layout first.
    let known = root
        .join("bin")
        .join("sdk")
        .join("sbf")
        .join("dependencies")
        .join("platform-tools")
        .join("rust")
        .join("bin");
    if known.join(HOST_CARGO).is_file() {
        return Some(known);
    }
    // Fall back to a bounded search under the bundled toolchain.
    find_dir_with(&root, HOST_CARGO, 8)
}

/// Depth-bounded search for the directory containing `file`.
fn find_dir_with(root: &Path, file: &str, depth: usize) -> Option<PathBuf> {
    if depth == 0 {
        return None;
    }
    if root.join(file).is_file() {
        return Some(root.to_path_buf());
    }
    for e in std::fs::read_dir(root).ok()?.flatten() {
        if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            if let Some(found) = find_dir_with(&e.path(), file, depth - 1) {
                return Some(found);
            }
        }
    }
    None
}

fn emit(app: &tauri::AppHandle, line: &str) {
    let _ = app.emit("build-log", line.to_string());
}
