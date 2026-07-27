// The core "translator": Rust smart-contract source  ->  SBF program (.so).
//
// Two shapes of input are accepted:
//   * A Cargo project (a folder with Cargo.toml, or the Cargo.toml itself).
//     We build it in place with `cargo build-sbf`.
//   * A single `.rs` file. We wrap it in a throwaway Cargo project (with a
//     solana-program dependency) so it can be compiled to SBF, then build that.
//
// The build output is the SBF ELF (`.so`) that the Pexli v2 runtime loads and
// executes — this is exactly the format an on-chain program ships in.

use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Serialize;
use sha2::{Digest, Sha256};
use tauri::{AppHandle, Emitter, Manager};

use crate::toolchain;

#[derive(Serialize)]
pub struct ConvertResult {
    pub so_path: String,
    pub out_dir: String,
    pub file_name: String,
    pub size: u64,
    pub sha256: String,
}

pub fn convert(
    app: &AppHandle,
    input_path: &str,
    output_dir: Option<String>,
) -> Result<ConvertResult, String> {
    // Check for the SBF compiler. If it is missing, set it up automatically so
    // a single click does the whole job: install the toolchain, then convert.
    let tool = match toolchain::resolve(app) {
        Some(t) => t,
        None => {
            emit(app, "SBF toolchain not found — installing it now (one-time setup)…");
            toolchain::install(app)?;
            toolchain::resolve(app).ok_or(
                "SBF toolchain setup did not finish. Please try again, or restart the app.",
            )?
        }
    };

    let input = PathBuf::from(input_path);
    if !input.exists() {
        return Err(format!("Input not found: {}", input.display()));
    }

    // Resolve the project we are going to build, plus an optional temp dir that
    // must outlive the build.
    let (project_dir, _scratch) = prepare_project(app, &input)?;

    emit(app, &format!("Project: {}", project_dir.display()));
    emit(app, "Starting SBF build (cargo build-sbf)…");

    run_build(app, &tool, &project_dir)?;

    // Locate the produced .so.
    let so = find_output(&project_dir)
        .ok_or("Build finished but no .so (SBF program) was produced.")?;
    emit(app, &format!("Found SBF program: {}", so.display()));

    // Copy to the chosen output folder (default: beside the input).
    let dest_dir = match output_dir {
        Some(d) => PathBuf::from(d),
        None => input
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from(".")),
    };
    fs::create_dir_all(&dest_dir).map_err(|e| format!("Cannot create output folder: {e}"))?;

    let file_name = so
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "program.so".into());
    let dest = dest_dir.join(&file_name);
    fs::copy(&so, &dest).map_err(|e| format!("Cannot copy output: {e}"))?;

    let bytes = fs::read(&dest).map_err(|e| format!("Cannot read output: {e}"))?;
    let size = bytes.len() as u64;
    let sha256 = {
        let mut h = Sha256::new();
        h.update(&bytes);
        format!("{:x}", h.finalize())
    };

    emit(app, &format!("Copied SBF program to {}", dest.display()));
    emit(app, "Done.");

    Ok(ConvertResult {
        so_path: dest.display().to_string(),
        out_dir: dest_dir.display().to_string(),
        file_name,
        size,
        sha256,
    })
}

/// Returns (project_dir_to_build, optional_tempdir_guard).
fn prepare_project(
    _app: &AppHandle,
    input: &Path,
) -> Result<(PathBuf, Option<tempfile::TempDir>), String> {
    // Case A: a Cargo.toml was selected -> build its parent folder.
    if input.file_name().map(|n| n == "Cargo.toml").unwrap_or(false) {
        let dir = input.parent().unwrap_or(Path::new(".")).to_path_buf();
        return Ok((dir, None));
    }
    // Case B: a folder that contains a Cargo.toml.
    if input.is_dir() && input.join("Cargo.toml").is_file() {
        return Ok((input.to_path_buf(), None));
    }
    // Case C: a single .rs file -> scaffold a throwaway project.
    if input.extension().map(|e| e == "rs").unwrap_or(false) {
        let scratch = tempfile::Builder::new()
            .prefix("pexliformation-")
            .tempdir()
            .map_err(|e| format!("Cannot create temp project: {e}"))?;
        scaffold_single_file(scratch.path(), input)?;
        let dir = scratch.path().to_path_buf();
        return Ok((dir, Some(scratch)));
    }
    Err("Unsupported input. Choose a .rs file, a Cargo.toml, or a Cargo project folder.".into())
}

/// Build a minimal Solana/Pexli program crate around a single lib.rs.
fn scaffold_single_file(dir: &Path, rs_file: &Path) -> Result<(), String> {
    let src = dir.join("src");
    fs::create_dir_all(&src).map_err(|e| e.to_string())?;
    let source = fs::read_to_string(rs_file).map_err(|e| format!("Cannot read source: {e}"))?;
    fs::write(src.join("lib.rs"), source).map_err(|e| e.to_string())?;

    let name = rs_file
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "pexli_program".into())
        .replace('-', "_");

    let cargo_toml = format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib", "lib"]

[dependencies]
solana-program = "1.18"

[profile.release]
overflow-checks = true
lto = "fat"
codegen-units = 1
"#
    );
    fs::write(dir.join("Cargo.toml"), cargo_toml).map_err(|e| e.to_string())?;
    Ok(())
}

/// Run `cargo build-sbf`, streaming every output line to the UI as it happens.
///
/// The build is fully self-contained and isolated: it uses the bundled
/// toolchain and an app-private CARGO_HOME, so it never installs anything and
/// never touches the user's own Rust/cargo setup.
fn run_build(app: &AppHandle, tool: &Path, project_dir: &Path) -> Result<(), String> {
    // `cargo-build-sbf` is invoked directly; it behaves like `cargo build-sbf`.
    let mut cmd = Command::new(tool);
    cmd.current_dir(project_dir);

    // Put the bundled toolchain's own bin dir first so its rust/llvm win over
    // anything on the system.
    if let Some(path) = std::env::var_os("PATH") {
        let mut paths: Vec<PathBuf> = std::env::split_paths(&path).collect();
        if let Some(bindir) = toolchain::bin_dir(tool) {
            paths.insert(0, bindir);
        }
        if let Ok(joined) = std::env::join_paths(paths) {
            cmd.env("PATH", joined);
        }
    }

    // App-private cargo home: crate downloads land inside our own data folder,
    // seeded once from the bundled cache so common contracts build offline.
    if let Some(cargo_home) = isolated_cargo_home(app) {
        emit(app, &format!("Using private toolchain cache: {}", cargo_home.display()));
        cmd.env("CARGO_HOME", &cargo_home);
    }

    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to start cargo build-sbf: {e}"))?;

    // Drain stderr on its own thread so a full pipe can't block stdout.
    let stderr = child.stderr.take();
    let app2 = app.clone();
    let joiner = std::thread::spawn(move || {
        if let Some(e) = stderr {
            for line in BufReader::new(e).lines().map_while(Result::ok) {
                let _ = app2.emit("build-log", line);
            }
        }
    });
    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            emit(app, &line);
        }
    }
    let _ = joiner.join();

    let status = child.wait().map_err(|e| e.to_string())?;
    if !status.success() {
        return Err(format!(
            "SBF build failed (exit {}). See the log for compiler errors.",
            status.code().unwrap_or(-1)
        ));
    }
    Ok(())
}

/// An app-private CARGO_HOME under the app's local data dir. Crate downloads go
/// here (never to the user's ~/.cargo), and the whole folder is removed on
/// uninstall. Seeded once from the bundled crate cache so the common
/// solana-program contract builds without any network access.
fn isolated_cargo_home(app: &AppHandle) -> Option<PathBuf> {
    let base = app.path().app_local_data_dir().ok()?;
    let cargo_home = base.join("toolchain").join("cargo");
    if !cargo_home.exists() {
        let _ = fs::create_dir_all(&cargo_home);
        if let Ok(res) = app.path().resource_dir() {
            let seed = res.join("resources").join("cargo-seed");
            if seed.is_dir() {
                emit(app, "Preparing the bundled crate cache (first run only)…");
                let _ = copy_dir_all(&seed, &cargo_home);
            }
        }
    }
    Some(cargo_home)
}

/// Recursively copy a directory tree.
fn copy_dir_all(src: &Path, dst: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let target = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Find the freshest `.so` under the usual SBF output folders.
fn find_output(project_dir: &Path) -> Option<PathBuf> {
    let candidates = [
        project_dir.join("target").join("deploy"),
        project_dir
            .join("target")
            .join("sbf-solana-solana")
            .join("release"),
    ];
    let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
    for dir in candidates {
        if let Ok(entries) = fs::read_dir(&dir) {
            for e in entries.flatten() {
                let p = e.path();
                if p.extension().map(|x| x == "so").unwrap_or(false) {
                    let mtime = e
                        .metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::UNIX_EPOCH);
                    if best.as_ref().map(|(t, _)| mtime > *t).unwrap_or(true) {
                        best = Some((mtime, p));
                    }
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

fn emit(app: &AppHandle, line: &str) {
    let _ = app.emit("build-log", line.to_string());
}
