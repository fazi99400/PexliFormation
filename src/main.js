// PexliFormation — frontend logic.
// Uses Tauri's global API (withGlobalTauri = true) so the UI needs no bundler.

const T = window.__TAURI__;
const invoke = T.core.invoke;
const listen = T.event.listen;
const dialogOpen = T.dialog.open;
const openPath = T.opener.openPath;
const getCurrentWebview = T.webview.getCurrentWebview;

const $ = (id) => document.getElementById(id);

let inputPath = null; // .rs file, Cargo.toml, or project folder
let outputDir = null; // chosen output folder (null = beside input)
let lastOutFolder = null;
let busy = false;

// ---- Toolchain status ----
async function checkToolchain() {
  try {
    const info = await invoke("check_toolchain");
    if (info.available) {
      $("tc-dot").className = "dot ok";
      $("tc-text").textContent = `SBF toolchain ready · ${info.version || "cargo-build-sbf"}`;
      $("toolchain").style.cursor = "default";
      $("toolchain").onclick = null;
    } else {
      $("tc-dot").className = "dot bad";
      $("tc-text").textContent = "SBF toolchain missing — click to set up";
      $("toolchain").style.cursor = "pointer";
      $("toolchain").onclick = installToolchain;
    }
  } catch (e) {
    $("tc-dot").className = "dot bad";
    $("tc-text").textContent = "Toolchain check failed";
  }
}

async function installToolchain() {
  if (busy) return;
  setBusy(true, "Setting up SBF toolchain (one-time)…");
  $("log").textContent = "";
  try {
    await invoke("install_toolchain");
    setStatus("Toolchain ready.");
    await checkToolchain();
  } catch (e) {
    setStatus("Toolchain setup failed: " + e);
  } finally {
    setBusy(false);
  }
}

// ---- Pick input ----
async function pickInput() {
  const sel = await dialogOpen({
    multiple: false,
    directory: false,
    filters: [{ name: "Rust contract", extensions: ["rs", "toml"] }],
  });
  if (sel) await setInput(sel);
}

async function setInput(path) {
  inputPath = path;
  $("input-name").textContent = path.split(/[\\/]/).pop();
  $("input-path").textContent = path;
  try {
    const preview = await invoke("read_source", { path });
    $("preview").textContent = preview;
  } catch (e) {
    $("preview").textContent = "// Could not read source: " + e;
  }
  $("btn-convert").disabled = false;
  setStatus("Input loaded. Ready to convert.");
}

// ---- Output dir ----
async function pickOutdir() {
  const sel = await dialogOpen({ directory: true, multiple: false });
  if (sel) {
    outputDir = sel;
    $("outdir-path").textContent = sel;
  }
}

// ---- Convert ----
async function convert() {
  if (!inputPath || busy) return;
  setBusy(true, "Converting Rust → SBF…");
  $("log").textContent = "";
  $("result").innerHTML = '<p class="muted">Building… please wait.</p>';
  try {
    const res = await invoke("convert_to_sbf", { inputPath, outputDir });
    lastOutFolder = res.out_dir;
    $("btn-open").disabled = false;
    $("result").innerHTML = `
      <div class="row"><span>Status</span><span class="ok-badge">Success</span></div>
      <div class="row"><span>SBF file</span><b>${esc(res.file_name)}</b></div>
      <div class="row"><span>Size</span><span>${(res.size / 1024).toFixed(1)} KB</span></div>
      <div class="row"><span>SHA-256</span><span class="mono">${esc(res.sha256)}</span></div>
      <div class="row"><span>Location</span><span class="mono">${esc(res.so_path)}</span></div>`;
    setStatus("Done — SBF program created.");
  } catch (e) {
    $("result").innerHTML = `<div class="row"><span>Status</span><span class="bad-badge">Failed</span></div>
      <p class="muted mono">${esc(String(e))}</p>`;
    setStatus("Conversion failed.");
  } finally {
    setBusy(false);
  }
}

// ---- Helpers ----
function esc(s) {
  return String(s).replace(/[&<>]/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;" })[c]);
}
function setBusy(state, msg) {
  busy = state;
  const btn = $("btn-convert");
  btn.disabled = state || !inputPath;
  btn.classList.toggle("busy", state);
  btn.innerHTML = state
    ? '<span class="spin"></span><span>Working…</span>'
    : '<span class="arrow">→</span><span>Convert to SBF</span>';
  if (msg) setStatus(msg);
}
function setStatus(s) {
  $("status").textContent = s;
}

// ---- Drag & drop (Tauri file drop) ----
async function initDrop() {
  const drop = $("drop");
  try {
    await getCurrentWebview().onDragDropEvent((ev) => {
      if (ev.payload.type === "over") drop.classList.add("hover");
      else if (ev.payload.type === "drop") {
        drop.classList.remove("hover");
        if (ev.payload.paths && ev.payload.paths.length) setInput(ev.payload.paths[0]);
      } else drop.classList.remove("hover");
    });
  } catch (_) {
    /* drag-drop optional */
  }
}

// ---- Live build log ----
listen("build-log", (e) => {
  const log = $("log");
  log.textContent += e.payload + "\n";
  log.scrollTop = log.scrollHeight;
});

// ---- Wire up ----
$("btn-pick").onclick = pickInput;
$("btn-outdir").onclick = pickOutdir;
$("btn-convert").onclick = convert;
$("btn-open").onclick = () => lastOutFolder && openPath(lastOutFolder);

checkToolchain();
initDrop();
