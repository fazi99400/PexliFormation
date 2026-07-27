# PexliFormation — one-time SBF toolchain setup for Windows.
#
# The app runs this automatically the first time you convert (or when you click
# the toolchain badge). It installs, only if missing:
#   1. Rust (rustup)                         — hosts the build
#   2. Agave/Solana CLI (cargo-build-sbf)    — the SBF compiler
#   3. platform-tools                        — the SBF LLVM/rustc (downloaded once)
# After this, PexliFormation converts fully offline.
#
# Can also be run by hand:
#   powershell -ExecutionPolicy Bypass -File scripts\setup-toolchain.ps1

$ErrorActionPreference = "Stop"
Write-Host "PexliFormation — SBF toolchain setup"

$cargoBin  = Join-Path $env:USERPROFILE ".cargo\bin"
$solanaBin = Join-Path $env:USERPROFILE ".local\share\solana\install\active_release\bin"
$env:Path  = "$cargoBin;$solanaBin;$env:Path"

# 1. Rust (rustup) — needed to host the SBF target.
if (-not (Get-Command rustc -ErrorAction SilentlyContinue)) {
    Write-Host "[1/3] Installing Rust (rustup)..."
    $rustup = Join-Path $env:TEMP "rustup-init.exe"
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustup
    & $rustup -y --profile minimal --default-toolchain stable
    $env:Path = "$cargoBin;$env:Path"
} else {
    Write-Host "[1/3] Rust already installed."
}

# 2. Agave / Solana CLI — ships cargo-build-sbf + platform-tools.
if (-not (Get-Command cargo-build-sbf -ErrorAction SilentlyContinue)) {
    Write-Host "[2/3] Installing SBF compiler (Agave)..."
    $init = Join-Path $env:TEMP "agave-install-init.exe"
    $url  = "https://release.anza.xyz/stable/agave-install-init-x86_64-pc-windows-msvc.exe"
    Invoke-WebRequest -Uri $url -OutFile $init
    # The init binary needs the release channel as an argument.
    & $init stable
    $env:Path = "$solanaBin;$env:Path"
} else {
    Write-Host "[2/3] SBF compiler already installed."
}

# 3. Warm up platform-tools (downloads the SBF LLVM/rustc on first run).
Write-Host "[3/3] Fetching platform-tools (one-time download)..."
cargo-build-sbf --version

Write-Host "Setup complete. PexliFormation is ready to convert offline."
