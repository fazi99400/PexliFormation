# PexliFormation

**Offline Windows app that turns a Rust smart contract into SBF — the format the Pexli v2 blockchain runs.**

Think of it as a *translator*: on the left you drop your Rust contract, you click one button, and on the right you get the `.so` (SBF program) ready for the chain. No terminal, no commands — just a simple window.

![Rust → SBF](https://img.shields.io/badge/Rust-%E2%86%92%20SBF-orange) ![Windows](https://img.shields.io/badge/Windows-one--click%20install-blue) ![Offline](https://img.shields.io/badge/works-offline-green)

---

## What it does

- **Input:** a single `.rs` contract file, a `Cargo.toml`, or a whole Cargo project folder.
- **Output:** a `program.so` in SBF format (the Solana/Pexli Berkeley-Packet-Filter bytecode the runtime executes), plus its size and SHA-256.
- **Offline:** once the SBF toolchain is set up on the machine, no internet is needed to convert.
- **Simple:** one window, drag-and-drop, one **Convert to SBF** button, live build log.

## Roman-Urdu quick note (aap ke liye)

- Ye ek **Windows software** hai — one-click install ho jata hai (`.exe` / `.msi`).
- Isme aap apna **Rust contract** dalen (jo Pexli v2 par chalta hai), aur **Convert** dabayen.
- Ye us ko **SBF format** (`.so`) me convert kar deta hai — bilkul kisi translator ki tarah.
- **Terminal ki zarurat nahi**, sab kuch UI se hota hai, aur **offline** chalta hai.
- Sirf **ek dafa** SBF toolchain setup karna hota hai (badge par click, ya `scripts\setup-toolchain.ps1`). Uske baad hamesha offline.

---

## Install (for users)

1. Download the installer from the project's **Releases** page (`PexliFormation_1.0.0_x64-setup.exe` or `.msi`).
2. Double-click → install (one click).
3. Open **PexliFormation**.
4. First run only: if the toolchain badge (top-right) is red, click it — it finishes the one-time SBF setup. After that it stays green and works offline.

## Use it

1. **Choose file / project** (or drag a `.rs` file onto the left panel).
2. (Optional) pick an **Output folder** — default is next to the input.
3. Click **Convert to SBF →**.
4. The right panel shows the resulting `.so`, its size and SHA-256. Click **Open folder** to reveal it.

Try the included [`examples/hello_pexli.rs`](examples/hello_pexli.rs).

---

## Build it yourself (for developers)

PexliFormation is a [Tauri v2](https://tauri.app) app — a Rust backend + a tiny build-free HTML/JS UI.

### Prerequisites
- [Rust](https://rustup.rs) (stable)
- [Node.js](https://nodejs.org) 18+
- Windows: the MSVC build tools + WebView2 (preinstalled on Win10/11)
- The SBF toolchain for actually converting: run [`scripts/setup-toolchain.ps1`](scripts/setup-toolchain.ps1)

### Commands
```bash
npm install
npm run dev          # run the app in dev mode
npm run build:win    # produce the one-click Windows installer (NSIS + MSI)
```
The installer lands in `src-tauri/target/release/bundle/`.

> Tip: an even easier route is the **GitHub Actions** workflow in
> [`.github/workflows/build-windows.yml`](.github/workflows/build-windows.yml) —
> push a tag like `v1.0.0` and it builds and attaches the Windows installer to a Release.

### Making it 100% offline (bundled toolchain)
Drop a copy of the Solana/Agave `platform-tools` into
`src-tauri/resources/platform-tools/` before building — see
[`src-tauri/resources/README.txt`](src-tauri/resources/README.txt). The app
prefers that bundled toolchain, so the installed software never touches the network.

---

## How the conversion works

```
 Rust source (.rs / Cargo project)
        │
        │  (single .rs → wrapped in a temp crate with solana-program)
        ▼
   cargo build-sbf        ← the SBF compiler (platform-tools)
        │
        ▼
 target/deploy/<name>.so  ← SBF program (this is your output)
```

The SBF `.so` is a small ELF the Pexli v2 / Solana runtime loads and executes on-chain. PexliFormation just drives that compiler for you behind a friendly window and copies the result where you want it.

## Project layout
```
src/                 UI (index.html, styles.css, main.js) — no bundler needed
src-tauri/           Rust backend (Tauri)
  src/lib.rs         Tauri commands
  src/converter.rs   Rust → SBF build logic
  src/toolchain.rs   finds / sets up cargo-build-sbf
  tauri.conf.json    window + Windows installer config
scripts/             icon generator, Windows toolchain setup
examples/            a sample contract
```

## License
MIT
