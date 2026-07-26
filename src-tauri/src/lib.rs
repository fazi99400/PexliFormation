// PexliFormation — offline Rust → SBF converter for the Pexli v2 blockchain.
//
// The Tauri backend exposes four commands to the UI:
//   check_toolchain   — is the SBF compiler present, and which version?
//   install_toolchain — one-time setup of the SBF platform-tools.
//   read_source       — preview the selected contract source.
//   convert_to_sbf    — the actual Rust -> SBF build.

mod converter;
mod toolchain;

use std::fs;

use tauri::AppHandle;

#[tauri::command]
fn check_toolchain(app: AppHandle) -> toolchain::ToolchainInfo {
    toolchain::info(&app)
}

#[tauri::command]
fn install_toolchain(app: AppHandle) -> Result<(), String> {
    toolchain::install(&app)
}

#[tauri::command]
fn read_source(path: String) -> Result<String, String> {
    let p = std::path::Path::new(&path);
    let file = if p.is_dir() {
        // Preview lib.rs / main.rs from a project folder.
        let lib = p.join("src").join("lib.rs");
        let main = p.join("src").join("main.rs");
        if lib.is_file() {
            lib
        } else if main.is_file() {
            main
        } else {
            p.join("Cargo.toml")
        }
    } else {
        p.to_path_buf()
    };
    let text = fs::read_to_string(&file).map_err(|e| e.to_string())?;
    // Cap the preview so the UI stays snappy on large files.
    const MAX: usize = 20_000;
    if text.len() > MAX {
        Ok(format!("{}\n\n… (truncated preview)", &text[..MAX]))
    } else {
        Ok(text)
    }
}

#[tauri::command]
fn convert_to_sbf(
    app: AppHandle,
    input_path: String,
    output_dir: Option<String>,
) -> Result<converter::ConvertResult, String> {
    converter::convert(&app, &input_path, output_dir)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            check_toolchain,
            install_toolchain,
            read_source,
            convert_to_sbf,
        ])
        .run(tauri::generate_context!())
        .expect("error while running PexliFormation");
}
