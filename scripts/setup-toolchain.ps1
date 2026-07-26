# PexliFormation — one-time SBF toolchain setup for Windows.
#
# PexliFormation itself is a normal desktop app. To actually compile Rust into
# SBF it needs the Solana/Agave "platform-tools" (which provides
# `cargo-build-sbf`). This script installs them once. After this, the app runs
# offline.
#
# Run in PowerShell:
#     powershell -ExecutionPolicy Bypass -File scripts\setup-toolchain.ps1

Write-Host "PexliFormation — SBF toolchain setup" -ForegroundColor Cyan

# 1. Rust (rustup) — needed to host the SBF target.
if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
    Write-Host "Installing Rust (rustup)..." -ForegroundColor Yellow
    $rustup = "$env:TEMP\rustup-init.exe"
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustup
    & $rustup -y
    $env:Path += ";$env:USERPROFILE\.cargo\bin"
} else {
    Write-Host "Rust already installed." -ForegroundColor Green
}

# 2. Agave / Solana CLI — ships cargo-build-sbf + platform-tools.
if (-not (Get-Command cargo-build-sbf -ErrorAction SilentlyContinue)) {
    Write-Host "Installing Agave/Solana CLI (provides cargo-build-sbf)..." -ForegroundColor Yellow
    $inst = "$env:TEMP\agave-install-init.exe"
    Invoke-WebRequest -Uri "https://release.anza.xyz/stable/agave-install-init-x86_64-pc-windows-msvc.exe" -OutFile $inst
    & $inst
    $env:Path += ";$env:USERPROFILE\.local\share\solana\install\active_release\bin"
} else {
    Write-Host "cargo-build-sbf already installed." -ForegroundColor Green
}

# 3. Warm up platform-tools (downloads the SBF LLVM/rustc once).
Write-Host "Warming up platform-tools..." -ForegroundColor Yellow
cargo-build-sbf --version

Write-Host "Done. Open PexliFormation — the toolchain badge should be green." -ForegroundColor Green
