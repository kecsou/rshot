# rshot Plan 2: Image Editor — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Clicking the floating thumbnail (or tray → Open Last Capture) opens the Glass image editor from mockup 03. It offers arrow, rectangle, ellipse, line, pen, highlighter, text, step counter, redact and crop, with undo/redo and zoom. **Done** writes the flattened PNG over the same file and re-copies the clipboard.

**Architecture:**
- Rust side: `editor.rs` adds commands on top of Plan 1's `AppState` and path guard. It opens one undecorated editor window per file and remembers `label → path`. `save_image` takes the flattened PNG as a **raw IPC body**, writes it atomically and re-copies the clipboard.
- Frontend: plain TS split into `model.ts` (pure document model, unit-tested), `render.ts` (Canvas 2D drawing and export), `styles.ts` (per-tool style state and popover) and `main.ts` (interaction).
- Shapes stay vectors in image-pixel coordinates until export. Redactions paint first, from the original pixels, then the other shapes in z-order.

**Tech Stack:** Plan 1's stack (Tauri 2.11, TypeScript 5.9, Vite 7.3, vitest 3.2). No new dependencies.

**Spec:** `docs/superpowers/specs/2026-09-22-rshot-design.md` §2.5 (and §2.3 thumbnail, §2.4 clipboard). Visual contract: `docs/design/mockups/03-editor.png`.

## Global Constraints

- All of Plan 1's Global Constraints and Environment notes still apply: `docs/superpowers/plans/2026-09-22-plan-1-foundation-linux-screenshots.md` (sections "Global Constraints" and "Environment notes").
- Tools and keys: select V, crop C, arrow A, rectangle R, ellipse O, line L, pen P, highlighter H, text T, step counter N, redact B.
- Colours: `#ff453a #ff9f0a #ffd60a #30d158 #0a84ff #bf5af2 #ffffff #1c1c1e`.
- Widths S/M/L = 3/5/8 image px (highlighter 14/22/34), times the image unit `min(2, max(1, min(W,H)/1080))`.
- Text sizes S/M/L/XL = 14/18/26/36 px times the unit. Text styles: filled, plain, outline.
- Redact modes: pixelate, blur, solid, plus a strength setting. Crop presets: Free, 16:9, 4:3, 1:1.
- **Done** / `Ctrl+S` / `⏎` saves in place (atomic) and re-copies the clipboard (same payload as a capture), then closes.
- **Copy** / `Ctrl+C` saves if the document is dirty and re-copies, but doesn't close.
- Closing with unsaved changes asks "Save changes?". `Esc` deselects first, then closes. Deleting asks for confirmation.
- Redaction must change the exported pixels (the original content can't be recovered from the saved PNG).
- The path in the clipboard never changes: the editor always saves over the file it opened.

## File Map

```
src-tauri/src/editor.rs            # open_editor, editor_info, save_image (raw body), editor_delete, copy_path
src-tauri/src/ui.rs                # + open_editor window; tray "Open Last Capture" → editor
src-tauri/src/thumbnail.rs         # guard() becomes pub(crate); open_capture removed
src-tauri/src/main.rs              # + mod editor, AppState.editors, handlers
src/shared/icons.ts                # + editor icons
src/shared/ipc.ts                  # + editor calls; − openCapture
src/thumbnail/{index.html,main.ts} # Edit button; click opens the editor
src/editor/index.html, editor.css  # mockup 03
src/editor/model.ts, model.test.ts # pure model
src/editor/render.ts               # canvas drawing + export
src/editor/styles.ts               # tool style state + popover
src/editor/main.ts                 # interaction
vite.config.ts                     # + 'editor' page
```

---

### Task 1: Editor window and commands, thumbnail/tray wiring, save round-trip

**Files:**
- Create: `src-tauri/src/editor.rs`, `src/editor/index.html` (final version), `src/editor/editor.css` (final version), `src/editor/main.ts` (minimal; Task 4 replaces it)
- Modify: `src-tauri/src/thumbnail.rs`, `src-tauri/src/ui.rs`, `src-tauri/src/main.rs`, `src/shared/icons.ts`, `src/shared/ipc.ts`, `src/thumbnail/index.html`, `src/thumbnail/main.ts`, `vite.config.ts`

**Interfaces:**
- Consumes (from Plan 1):
  - `AppState { config, clipboard, last_capture, thumb, … }` and `thumbnail::{guard (made pub(crate)), tildify}`.
  - `store::{write_atomic, ClipboardMode}`, `Clipboard::copy_capture(&Path, Option<&[u8]>, ClipboardMode)`.
  - `ui::{close_prefix, monitor_at, POPUP_SEQ}` and `settings::close_window`.
- Produces:
  - Commands:
    - `open_editor(path)`
    - `editor_info() -> { path, display, name }`
    - `save_image(<raw PNG body>)`
    - `editor_delete()`
    - `copy_path(path)`
  - `ui::open_editor(&AppHandle, &Path) -> Result<(), String>` and `AppState.editors: Mutex<HashMap<String, PathBuf>>`.
  - TS: `ipc.openEditor`, `ipc.editorInfo`, `ipc.saveImage(Uint8Array)`, `ipc.editorDelete`, `ipc.copyPath` and `type EditorInfo`.

- [ ] **Step 1: Make the path guard reusable, let it accept open editor files, and drop the default-viewer command**

Plan 1's `thumbnail::guard` accepts only the canonicalized **last capture**. It goes through a pure `allowed(p, last)` helper, a security ruling from Plan 1 Task 9. Editors must keep working after another capture replaces "last". So:
- make `guard` `pub(crate)`;
- extend it to also accept a path currently open in an editor window. Before the final `Err`, check `state.editors.lock().unwrap().values().any(|e| e.canonicalize().ok().as_deref() == Some(p.as_path()))`. `AppState.editors` is added in Step 4, so do this edit after Step 4 compiles;
- add a unit test that covers "open in an editor ⇒ allowed" by extending the pure helper (e.g. `allowed(p, last, open: &[PathBuf])`).

Delete the `open_capture` command, and remove it from `generate_handler!` in `main.rs` and from `src/shared/ipc.ts` (`openCapture`).

- [ ] **Step 2: Write `src-tauri/src/editor.rs`**

```rust
//! Commands behind the image editor window. The window label maps to the file it edits.

use crate::{err, store, thumbnail, ui, AppState};
use serde::Serialize;
use std::path::PathBuf;
use tauri::{
    ipc::{InvokeBody, Request},
    AppHandle, State, WebviewWindow,
};

#[derive(Serialize)]
pub struct EditorInfo {
    path: String,
    display: String,
    name: String,
}

fn editor_path(state: &AppState, window: &WebviewWindow) -> Result<PathBuf, String> {
    state.editors.lock().unwrap().get(window.label()).cloned().ok_or_else(|| "not an editor window".to_string())
}

/// async: building a window from a sync command deadlocks on Windows (WebView2).
#[tauri::command]
pub async fn open_editor(app: AppHandle, state: State<'_, AppState>, path: String) -> Result<(), String> {
    let p = thumbnail::guard(&state, &path)?;
    ui::close_prefix(&app, "thumbnail");
    ui::open_editor(&app, &p)
}

#[tauri::command]
pub fn editor_info(window: WebviewWindow, state: State<'_, AppState>) -> Result<EditorInfo, String> {
    let p = editor_path(&state, &window)?;
    Ok(EditorInfo {
        path: p.display().to_string(),
        display: thumbnail::tildify(&p),
        name: p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
    })
}

/// Body = the flattened PNG. Overwrites the capture atomically and re-copies the clipboard,
/// so the path the user already pasted keeps pointing at what they see.
/// async: writes up to ~16 MB and waits on the clipboard thread — keep it off the GTK main thread.
#[tauri::command]
pub async fn save_image(window: WebviewWindow, state: State<'_, AppState>, request: Request<'_>) -> Result<(), String> {
    let InvokeBody::Raw(png) = request.body() else {
        return Err("expected PNG bytes".into());
    };
    let p = editor_path(&state, &window)?;
    store::write_atomic(&p, png).map_err(err)?;
    let mode = state.config.lock().unwrap().clipboard_mode;
    state.clipboard.copy_capture(&p, Some(png), mode)
}

#[tauri::command]
pub fn editor_delete(window: WebviewWindow, state: State<'_, AppState>) -> Result<(), String> {
    let p = editor_path(&state, &window)?;
    std::fs::remove_file(&p).map_err(err)?;
    state.editors.lock().unwrap().remove(window.label());
    // Don't leave the tray / thumbnail pointing at a deleted file (Plan 1 final review).
    let mut last = state.last_capture.lock().unwrap();
    if last.as_ref().and_then(|l| l.canonicalize().ok()).is_none() {
        *last = None;
        state.thumb.lock().unwrap().take();
    }
    drop(last);
    window.destroy().map_err(err)
}

/// The path as text only (footer chip).
#[tauri::command]
pub fn copy_path(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let p = thumbnail::guard(&state, &path)?;
    state.clipboard.copy_capture(&p, None, store::ClipboardMode::PathOnly)
}
```

- [ ] **Step 3: Add `ui::open_editor` and route the tray's "Open Last Capture" to it**

In `ui.rs`, make `monitor_at` visible to the module if it isn't already (it's in the same file). Then add:

```rust
/// One editor per file: refocus it if it's open, otherwise open one at 80 % of the monitor under the pointer.
pub fn open_editor(app: &AppHandle, path: &std::path::Path) -> Result<(), String> {
    let state = app.state::<crate::AppState>();
    let existing = {
        let mut editors = state.editors.lock().unwrap();
        editors.retain(|label, _| app.get_webview_window(label).is_some());
        editors.iter().find(|(_, p)| p.as_path() == path).map(|(label, _)| label.clone())
    };
    if let Some(w) = existing.and_then(|label| app.get_webview_window(&label)) {
        w.show().map_err(err)?;
        return w.set_focus().map_err(err);
    }
    let cursor = app.cursor_position().map_err(err)?;
    let m = monitor_at(app, cursor.x, cursor.y)?;
    let s = m.scale_factor();
    let (mw, mh) = (m.size().width as f64 / s, m.size().height as f64 / s);
    let (w, h) = ((mw * 0.8).max(800.0), (mh * 0.8).max(560.0));
    let label = format!("editor-{}", POPUP_SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
    state.editors.lock().unwrap().insert(label.clone(), path.to_path_buf());
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let win = WebviewWindowBuilder::new(app, &label, WebviewUrl::App("editor/index.html".into()))
        .title(format!("{name} — rshot"))
        .decorations(false)
        .transparent(true)
        .resizable(true)
        .min_inner_size(800.0, 560.0)
        .inner_size(w, h)
        .visible(false)
        .build()
        .map_err(err)?;
    let x = m.position().x + ((m.size().width as f64 - w * s) / 2.0) as i32;
    let y = m.position().y + ((m.size().height as f64 - h * s) / 2.0) as i32;
    win.set_position(PhysicalPosition::new(x, y)).map_err(err)?;
    win.show().map_err(err)?;
    win.set_focus().map_err(err)?;
    // Mutter ignores set_focus for windows opened from another app's click (Plan 1 Task 6/13).
    #[cfg(target_os = "linux")]
    force_focus(&win);
    Ok(())
}
```

Replace the body of `open_last` in `ui.rs`:

```rust
fn open_last(app: &AppHandle) -> Result<(), String> {
    let last = app.state::<crate::AppState>().last_capture.lock().unwrap().clone();
    let path = last.ok_or("No capture yet")?;
    if !path.exists() {
        return Err("The last capture no longer exists".into());
    }
    open_editor(app, &path)
}
```

(Remove the now-unused `OpenerExt` import from `open_last` if the compiler warns.)

- [ ] **Step 4: Wire `main.rs`**

- Add `mod editor;`.
- Add `pub editors: std::sync::Mutex<std::collections::HashMap<String, std::path::PathBuf>>,` to `AppState` and `editors: std::sync::Mutex::new(std::collections::HashMap::new()),` to `new()`.
- Add `editor::open_editor, editor::editor_info, editor::save_image, editor::editor_delete, editor::copy_path,` to `generate_handler!`.

- [ ] **Step 5: Add the editor icons to `src/shared/icons.ts`**

Insert these symbols inside `SPRITE`, before `</svg>`:

```html
<symbol id="t-select" viewBox="0 0 24 24"><path d="M6 3l12 8-5.5 1.3L9.5 18z"/></symbol>
<symbol id="t-crop" viewBox="0 0 24 24"><path d="M6 2v14a2 2 0 0 0 2 2h14M2 6h14a2 2 0 0 1 2 2v14"/></symbol>
<symbol id="t-arrow" viewBox="0 0 24 24"><path d="M5 19L19 5M10 5h9v9"/></symbol>
<symbol id="t-rect" viewBox="0 0 24 24"><rect x="4" y="5" width="16" height="14" rx="2"/></symbol>
<symbol id="t-ellipse" viewBox="0 0 24 24"><ellipse cx="12" cy="12" rx="9" ry="7"/></symbol>
<symbol id="t-line" viewBox="0 0 24 24"><path d="M5 19L19 5"/></symbol>
<symbol id="t-pen" viewBox="0 0 24 24"><path d="M3 18c2.5-5 4.5-9 6.5-9s1 6 3.5 6 3-7 5-7 2 3 3 4"/></symbol>
<symbol id="t-hl" viewBox="0 0 24 24"><path d="M14.5 4.5l5 5L11 18H6v-5z"/><path d="M3 21h9" stroke-width="3"/></symbol>
<symbol id="t-text" viewBox="0 0 24 24"><path d="M5 7V5h14v2M12 5v14M9 19h6"/></symbol>
<symbol id="t-num" viewBox="0 0 24 24"><circle cx="12" cy="12" r="9"/><path d="M10.5 9.5L12.5 8v8"/></symbol>
<symbol id="t-blur" viewBox="0 0 24 24"><rect x="4" y="4" width="5" height="5" rx="1"/><rect x="15" y="4" width="5" height="5" rx="1" fill="currentColor"/><rect x="9.5" y="9.5" width="5" height="5" rx="1" fill="currentColor"/><rect x="4" y="15" width="5" height="5" rx="1" fill="currentColor"/><rect x="15" y="15" width="5" height="5" rx="1"/></symbol>
<symbol id="i-undo" viewBox="0 0 24 24"><path d="M9 14L4 9l5-5M4 9h10.5a5.5 5.5 0 0 1 0 11H11"/></symbol>
<symbol id="i-redo" viewBox="0 0 24 24"><path d="M15 14l5-5-5-5M20 9H9.5a5.5 5.5 0 0 0 0 11H13"/></symbol>
<symbol id="i-copy" viewBox="0 0 24 24"><rect x="8" y="8" width="12" height="12" rx="2"/><path d="M16 8V6a2 2 0 0 0-2-2H6a2 2 0 0 0-2 2v8a2 2 0 0 0 2 2h2"/></symbol>
<symbol id="i-minus" viewBox="0 0 24 24"><path d="M6 12h12"/></symbol>
<symbol id="i-plus" viewBox="0 0 24 24"><path d="M12 6v12M6 12h12"/></symbol>
<symbol id="i-pen" viewBox="0 0 24 24"><path d="M4 20h4L19 9l-4-4L4 16z"/></symbol>
<symbol id="e-one" viewBox="0 0 24 24"><path d="M4 12h14M13 7l5 5-5 5"/></symbol>
<symbol id="e-two" viewBox="0 0 24 24"><path d="M4 12h16M8 8l-4 4 4 4M16 8l4 4-4 4"/></symbol>
<symbol id="e-none" viewBox="0 0 24 24"><path d="M4 12h16"/></symbol>
```

- [ ] **Step 6: Add the IPC calls**

Append to `src/shared/ipc.ts`:

```ts
export type EditorInfo = { path: string; display: string; name: string };
export const openEditor = (path: string) => invoke<void>('open_editor', { path });
export const editorInfo = () => invoke<EditorInfo>('editor_info');
/** Sends the PNG as a raw IPC body (no JSON/base64 round-trip). */
export const saveImage = (png: Uint8Array) => invoke<void>('save_image', png);
export const editorDelete = () => invoke<void>('editor_delete');
export const copyPath = (path: string) => invoke<void>('copy_path', { path });
```

- [ ] **Step 7: Send the thumbnail to the editor**

In `src/thumbnail/index.html`, make this the first button inside `.acts`:

```html
<button data-act="edit" data-icon="i-pen" title="Edit" aria-label="Edit"></button>
```

In `src/thumbnail/main.ts`:
- In the click handler, add `if (b.dataset.act === 'edit') await ipc.openEditor(t.path);` as the first branch.
- In `onUp`, replace `void ipc.openCapture(t.path)` with `void ipc.openEditor(t.path)`.

- [ ] **Step 8: Write the final editor HTML and CSS, plus a minimal `main.ts` for the round-trip**

`src/editor/index.html`:

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>rshot editor</title>
  </head>
  <body data-tool="arrow">
    <div class="ed">
      <header data-tauri-drag-region>
        <span class="title" id="title"></span><span class="meta" id="meta"></span><span class="sp" data-tauri-drag-region></span>
        <button class="ib" id="undo" data-icon="i-undo" title="Undo (Ctrl+Z)" aria-label="Undo"></button>
        <button class="ib" id="redo" data-icon="i-redo" title="Redo (Ctrl+Shift+Z)" aria-label="Redo"></button>
        <span class="vsep"></span>
        <button class="ib" id="delete" data-icon="i-trash" title="Delete capture" aria-label="Delete capture"></button>
        <button class="b2" id="copy" data-icon="i-copy" title="Copy path + image (Ctrl+C)">Copy</button>
        <button class="b1" id="done" title="Save and close (Ctrl+S)">Done</button>
        <button class="wx" id="close" data-icon="i-x" aria-label="Close"></button>
      </header>
      <div class="body">
        <div class="stage" id="stage"><canvas id="view"></canvas><textarea id="texted" hidden spellcheck="false"></textarea></div>
        <div class="rail glass" id="rail">
          <button data-tool="select" data-icon="t-select" title="Select (V)" aria-label="Select"></button>
          <button data-tool="crop" data-icon="t-crop" title="Crop (C)" aria-label="Crop"></button>
          <span class="hsep"></span>
          <button data-tool="arrow" data-icon="t-arrow" title="Arrow (A)" aria-label="Arrow"></button>
          <button data-tool="rect" data-icon="t-rect" title="Rectangle (R)" aria-label="Rectangle"></button>
          <button data-tool="ellipse" data-icon="t-ellipse" title="Ellipse (O)" aria-label="Ellipse"></button>
          <button data-tool="line" data-icon="t-line" title="Line (L)" aria-label="Line"></button>
          <button data-tool="pen" data-icon="t-pen" title="Pen (P)" aria-label="Pen"></button>
          <button data-tool="highlight" data-icon="t-hl" title="Highlighter (H)" aria-label="Highlighter"></button>
          <button data-tool="text" data-icon="t-text" title="Text (T)" aria-label="Text"></button>
          <button data-tool="counter" data-icon="t-num" title="Step counter (N)" aria-label="Step counter"></button>
          <button data-tool="redact" data-icon="t-blur" title="Redact (B)" aria-label="Redact"></button>
          <span class="hsep"></span>
          <button id="swatch" title="Style" aria-label="Style"><i></i></button>
        </div>
        <div id="pop" class="glass" hidden></div>
        <div id="cropbar" class="glass" hidden>
          <div class="seg" id="ratios"><button data-ratio="free">Free</button><button data-ratio="16:9">16:9</button><button data-ratio="4:3">4:3</button><button data-ratio="1:1">1:1</button></div>
          <span class="dimv" id="cropdim"></span>
          <button class="b2" id="crop-cancel">Cancel</button>
          <button class="b1" id="crop-apply">Apply ⏎</button>
        </div>
        <div id="modal" hidden><div class="glass box"><p id="modal-text"></p><div class="acts" id="modal-acts"></div></div></div>
      </div>
      <footer>
        <div class="chip">
          <span class="ok"><svg class="ic" aria-hidden="true"><use href="#i-check" /></svg></span><span class="p" id="path"></span>
          <button class="cp" id="copy-path" data-icon="i-copy" title="Copy path" aria-label="Copy path"></button>
          <button class="cp" id="reveal" data-icon="i-folder" title="Show in folder" aria-label="Show in folder"></button>
        </div>
        <div class="zoom">
          <span class="dirty" id="dirty" hidden><i></i>Unsaved changes</span>
          <button class="ib" id="zoom-out" data-icon="i-minus" title="Zoom out (Ctrl+-)" aria-label="Zoom out"></button>
          <span id="zoom-val"></span>
          <button class="ib" id="zoom-in" data-icon="i-plus" title="Zoom in (Ctrl++)" aria-label="Zoom in"></button>
          <button class="b2" id="fit" title="Fit (Ctrl+0)">Fit</button>
        </div>
      </footer>
    </div>
    <script type="module" src="./main.ts"></script>
  </body>
</html>
```

`src/editor/editor.css`:

```css
html, body { height: 100%; overflow: hidden; }
.ed { height: 100%; display: flex; flex-direction: column; border-radius: 12px; overflow: hidden; background: var(--surface); border: 1px solid #34343a; }
header { height: 48px; flex-shrink: 0; background: var(--surface-2); border-bottom: 1px solid var(--line); display: flex; align-items: center; padding: 0 10px 0 16px; gap: 8px; }
.title { font: 600 13px Inter; white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.meta { font: 500 11.5px Inter; color: var(--text-3); margin-left: 8px; white-space: nowrap; }
.sp { flex: 1; align-self: stretch; }
.ib { width: 32px; height: 30px; display: grid; place-items: center; border-radius: 8px; color: #d9d9e0; }
.ib:hover { background: rgba(255, 255, 255, 0.1); }
.ib:disabled { color: rgba(255, 255, 255, 0.3); background: none; }
.vsep { width: 1px; height: 20px; background: rgba(255, 255, 255, 0.12); margin: 0 4px; }
.wx { width: 24px; height: 24px; border-radius: 50%; display: grid; place-items: center; background: rgba(255, 255, 255, 0.1); margin-left: 8px; }
.wx .ic { width: 12px; height: 12px; }

.body { flex: 1; position: relative; min-height: 0; }
.stage { position: absolute; inset: 0; overflow: auto; display: grid; place-items: center; padding: 30px 30px 30px 90px; background: radial-gradient(circle, rgba(255, 255, 255, 0.07) 1px, transparent 1.2px) 0 0 / 18px 18px, var(--surface); }
#view { display: block; box-shadow: 0 0 0 1px rgba(255, 255, 255, 0.06), 0 24px 60px rgba(0, 0, 0, 0.55); border-radius: 3px; cursor: crosshair; touch-action: none; }
body[data-tool='select'] #view { cursor: default; }
body[data-tool='text'] #view { cursor: text; }

.rail { position: absolute; left: 14px; top: 50%; transform: translateY(-50%); display: flex; flex-direction: column; gap: 2px; padding: 6px; border-radius: 14px; z-index: 5; background: rgba(44, 44, 50, 0.78); }
.rail button[data-tool] { width: 36px; height: 34px; display: grid; place-items: center; border-radius: 9px; color: #d9d9e0; }
.rail button[data-tool]:hover { background: rgba(255, 255, 255, 0.1); }
.rail button[data-tool].on { background: var(--accent); color: #fff; }
.rail .ic { width: 18px; height: 18px; }
.rail .hsep { height: 1px; background: rgba(255, 255, 255, 0.12); margin: 4px 6px; }
#swatch { width: 36px; height: 34px; display: grid; place-items: center; }
#swatch i { width: 18px; height: 18px; border-radius: 50%; box-shadow: 0 0 0 2px rgba(255, 255, 255, 0.9); display: block; }

#pop { position: absolute; left: 66px; width: 232px; padding: 12px; border-radius: 14px; z-index: 6; font: 500 12px Inter; background: rgba(44, 44, 50, 0.78); }
#pop .lbl { margin: 0 0 7px; }
#pop .sws { display: grid; grid-template-columns: repeat(8, 1fr); gap: 6px; margin-bottom: 12px; }
#pop .sws button { aspect-ratio: 1; border-radius: 50%; box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.15); }
#pop .sws button.on { box-shadow: 0 0 0 2px #232328, 0 0 0 4px #fff; }
#pop .seg { margin-bottom: 12px; }
#pop .seg button { height: 26px; display: grid; place-items: center; padding: 0; }
#pop .seg b { display: block; width: 22px; border-radius: 3px; background: currentColor; }
#pop .tog { display: flex; align-items: center; justify-content: space-between; height: 24px; }
#pop input[type='range'] { width: 100%; accent-color: var(--accent); margin: 4px 0 12px; }
#pop .note { font: 500 11px Inter; color: var(--text-2); line-height: 1.5; }

#texted { position: absolute; z-index: 4; resize: none; overflow: hidden; border: 1px dashed var(--accent); outline: none; background: transparent; color: var(--c); padding: 0; line-height: 1.25; white-space: pre; font-weight: 600; }
#texted[data-style='filled'] { background: var(--c); color: var(--ink); padding: 0.35em 0.6em; border-radius: 0.4em; }
#texted[data-style='outline'] { color: var(--ink); -webkit-text-stroke: 0.11em var(--c); paint-order: stroke fill; }

#cropbar { position: absolute; left: 50%; bottom: 16px; transform: translateX(-50%); display: flex; gap: 6px; align-items: center; padding: 5px; border-radius: 11px; font: 500 11.5px Inter; white-space: nowrap; z-index: 6; }
#cropbar .dimv { padding: 0 8px; min-width: 90px; text-align: center; }
#cropbar .seg button { padding: 4px 8px; }

footer { height: 40px; flex-shrink: 0; background: var(--surface-2); border-top: 1px solid var(--line); display: flex; align-items: center; justify-content: space-between; padding: 0 10px 0 12px; font: 500 11.5px Inter; }
.chip { display: flex; align-items: center; gap: 8px; height: 26px; padding: 0 6px 0 8px; border-radius: 7px; background: rgba(255, 255, 255, 0.06); min-width: 0; max-width: 60%; }
.chip .ok { width: 15px; height: 15px; border-radius: 50%; background: var(--ok); display: grid; place-items: center; color: #063; flex-shrink: 0; }
.chip .ok .ic { width: 9px; height: 9px; stroke-width: 3.2; }
.chip .p { color: rgba(255, 255, 255, 0.75); white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
.chip .cp { width: 22px; height: 22px; border-radius: 6px; display: grid; place-items: center; color: rgba(255, 255, 255, 0.7); flex-shrink: 0; }
.chip .cp .ic { width: 14px; height: 14px; }
.zoom { display: flex; align-items: center; gap: 2px; color: rgba(255, 255, 255, 0.8); }
.zoom .ib { width: 26px; height: 24px; }
.zoom .ib .ic { width: 14px; height: 14px; }
#zoom-val { width: 44px; text-align: center; }
#fit { height: 24px; margin-left: 6px; font-weight: 500; }
.dirty { color: var(--text-3); margin-right: 12px; }
.dirty i { display: inline-block; width: 6px; height: 6px; border-radius: 50%; background: #ff9f0a; margin-right: 6px; vertical-align: 1px; }

#modal { position: absolute; inset: 0; z-index: 20; display: grid; place-items: center; background: rgba(0, 0, 0, 0.45); }
#modal .box { width: 360px; padding: 20px; border-radius: 14px; font: 500 13px Inter; line-height: 1.5; }
#modal .acts { display: flex; justify-content: flex-end; gap: 8px; margin-top: 16px; }
.danger { background: var(--danger) !important; color: #fff; }
```

`src/editor/main.ts` (minimal: shows the capture, and Done re-saves it unchanged; Task 4 replaces it):

```ts
import '../shared/base';
import '../shared/glass.css';
import './editor.css';
import { mountIcons } from '../shared/icons';
import * as ipc from '../shared/ipc';

// Minimal round-trip editor; Task 4 replaces this file.
mountIcons();
const info = await ipc.editorInfo();
const img = await createImageBitmap(new Blob([await ipc.readCapture(info.path)], { type: 'image/png' }));
const canvas = document.querySelector<HTMLCanvasElement>('#view')!;
canvas.width = img.width;
canvas.height = img.height;
canvas.style.width = `${Math.min(img.width, 900)}px`;
canvas.getContext('2d')!.drawImage(img, 0, 0);
document.querySelector('#title')!.textContent = info.name;
document.querySelector('#path')!.textContent = info.display;
document.querySelector('#done')!.addEventListener('click', async () => {
  const blob = await new Promise<Blob>((ok) => canvas.toBlob((b) => ok(b!), 'image/png'));
  await ipc.saveImage(new Uint8Array(await blob.arrayBuffer()));
  await ipc.closeWindow();
});
document.querySelector('#close')!.addEventListener('click', () => void ipc.closeWindow());
```

In `vite.config.ts`, add `'editor'` to `pages`.

- [ ] **Step 9: Verify the round-trip**

Run: `npm run build && npm test && (cd src-tauri && cargo clippy --all-targets -- -D warnings && cargo test)`
Expected: everything passes with no warnings.

Then, with `npm run tauri dev` running:
1. Run `./src-tauri/target/debug/rshot capture screen`. The thumbnail appears; hovering shows **Edit** (pencil), folder and trash.
2. Click the thumbnail. The editor window opens, 80 % of the monitor, centred, showing the capture, with the title and footer path set.
3. `stat -c %Y <path>`, then click **Done**. The window closes, the file's mtime changes, the file is still a valid PNG of the same size (`python3 -c "from PIL import Image; print(Image.open('<path>').size)"`), and `xclip -selection clipboard -o` prints the path.
4. Tray → **Open Last Capture** opens the editor for that file. Running it again while the editor is open focuses the same window; no second one opens.

- [ ] **Step 10: Commit**

```bash
git add -A src src-tauri vite.config.ts && git commit -m "feat(editor): editor window, raw-body save round-trip, thumbnail and tray entry points

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 2: Document model (`model.ts`), test-first

**Files:**
- Create: `src/editor/model.ts`, `src/editor/model.test.ts`

**Interfaces:**
- Produces, in `model.ts` (all pure):
  - Types: `Pt`, `Rect`, `Width` (0|1|2), `TextSize`, `TextStyle`, `Ends`, `RedactMode`, `Shape` (discriminated by `kind`), `Doc { shapes, crop }`, `HandleId`, `Ratio`.
  - Constants: `WIDTHS`, `HIGHLIGHT_WIDTHS`, `TEXT_PX`, `COUNTER_R`, `SWATCHES`, `ZOOMS`, `RATIOS`.
  - Functions:

    ```ts
    unitFor(w, h)
    norm(a, b)
    strokeWidth(s, u)
    bounds(s, u)
    distToSeg(p, a, b)
    hitShape(s, p, tol, u)
    hitTest(doc, p, tol, u)
    translate(s, dx, dy)
    handles(s)
    dragHandle(s, h, p)
    snap45(a, b)
    squareEnd(a, b)
    nextCounter(doc)
    exportOrder(doc)
    fitZoom(w, h, vw, vh)
    zoomStep(z, dir)
    fitRatio(r, ratio)
    cropFromDrag(a, b, ratio, W, H)
    viewToImage(p, zoom, crop)
    ```

  - `class History { doc; commit(d); undo(); redo(); markSaved(); canUndo; canRedo; dirty }`.

- [ ] **Step 1: Write the failing tests**

`src/editor/model.test.ts`:

```ts
import { describe, expect, it } from 'vitest';
import * as M from './model';

const arrow = (id: number, a: M.Pt, b: M.Pt): M.Shape => ({ id, kind: 'arrow', a, b, color: '#ff453a', width: 1, ends: 'one', shadow: true });
const rect = (id: number, a: M.Pt, b: M.Pt): M.Shape => ({ id, kind: 'rect', a, b, color: '#ff453a', width: 1 });
const counter = (id: number, n: number): M.Shape => ({ id, kind: 'counter', at: { x: 50, y: 50 }, n, color: '#ff453a' });
const redact = (id: number): M.Shape => ({ id, kind: 'redact', a: { x: 0, y: 0 }, b: { x: 10, y: 10 }, mode: 'pixelate', strength: 0.5 });
const doc = (...shapes: M.Shape[]): M.Doc => ({ shapes, crop: null });

describe('geometry', () => {
  it('unitFor scales from 1080p, capped at 2', () => {
    expect(M.unitFor(1920, 1080)).toBe(1);
    expect(M.unitFor(800, 600)).toBe(1);
    expect(M.unitFor(3840, 2160)).toBe(2);
  });
  it('norm and distToSeg', () => {
    expect(M.norm({ x: 10, y: 5 }, { x: 2, y: 9 })).toEqual({ x: 2, y: 5, w: 8, h: 4 });
    expect(M.distToSeg({ x: 5, y: 3 }, { x: 0, y: 0 }, { x: 10, y: 0 })).toBe(3);
    expect(M.distToSeg({ x: 13, y: 4 }, { x: 0, y: 0 }, { x: 10, y: 0 })).toBe(5);
  });
  it('bounds covers pen strokes and counters', () => {
    const pen: M.Shape = { id: 1, kind: 'pen', pts: [{ x: 5, y: 9 }, { x: 1, y: 2 }, { x: 8, y: 4 }], color: '#fff', width: 0 };
    expect(M.bounds(pen)).toEqual({ x: 1, y: 2, w: 7, h: 7 });
    expect(M.bounds(counter(2, 1), 2)).toEqual({ x: 50 - 26, y: 50 - 26, w: 52, h: 52 });
  });
});

describe('hit testing', () => {
  it('hits an arrow near its shaft only', () => {
    const s = arrow(1, { x: 0, y: 0 }, { x: 100, y: 0 });
    expect(M.hitShape(s, { x: 50, y: 4 }, 3)).toBe(true);
    expect(M.hitShape(s, { x: 50, y: 20 }, 3)).toBe(false);
  });
  it('hits a rectangle on its border, not its inside', () => {
    const s = rect(1, { x: 0, y: 0 }, { x: 100, y: 50 });
    expect(M.hitShape(s, { x: 1, y: 25 }, 3)).toBe(true);
    expect(M.hitShape(s, { x: 50, y: 25 }, 3)).toBe(false);
  });
  it('hitTest returns the top-most shape', () => {
    const d = doc(rect(1, { x: 0, y: 0 }, { x: 100, y: 100 }), arrow(2, { x: 0, y: 0 }, { x: 100, y: 0 }));
    expect(M.hitTest(d, { x: 50, y: 1 }, 3)?.id).toBe(2);
    expect(M.hitTest(d, { x: 1, y: 50 }, 3)?.id).toBe(1);
    expect(M.hitTest(d, { x: 50, y: 50 }, 3)).toBeNull();
  });
});

describe('editing', () => {
  it('translate moves every point', () => {
    expect(M.translate(arrow(1, { x: 0, y: 0 }, { x: 10, y: 10 }), 5, -2)).toMatchObject({ a: { x: 5, y: -2 }, b: { x: 15, y: 8 } });
  });
  it('dragHandle keeps the opposite corner fixed', () => {
    const s = rect(1, { x: 10, y: 10 }, { x: 50, y: 40 });
    expect(M.bounds(M.dragHandle(s, 'nw', { x: 0, y: 0 }))).toEqual({ x: 0, y: 0, w: 50, h: 40 });
    expect(M.bounds(M.dragHandle(s, 'se', { x: 60, y: 60 }))).toEqual({ x: 10, y: 10, w: 50, h: 50 });
    expect(M.dragHandle(arrow(2, { x: 0, y: 0 }, { x: 5, y: 5 }), 'b', { x: 9, y: 1 })).toMatchObject({ b: { x: 9, y: 1 } });
  });
  it('snap45 and squareEnd constrain with Shift', () => {
    expect(M.snap45({ x: 0, y: 0 }, { x: 10, y: 1 })).toEqual({ x: 10, y: 0 });
    expect(M.snap45({ x: 0, y: 0 }, { x: 10, y: 9 })).toEqual({ x: 10, y: 10 });
    expect(M.squareEnd({ x: 0, y: 0 }, { x: -3, y: 8 })).toEqual({ x: -8, y: 8 });
  });
  it('nextCounter continues after the highest number', () => {
    expect(M.nextCounter(doc())).toBe(1);
    expect(M.nextCounter(doc(counter(1, 1), counter(2, 4)))).toBe(5);
  });
  it('exportOrder paints redactions first, keeping the rest in z-order', () => {
    const d = doc(arrow(1, { x: 0, y: 0 }, { x: 1, y: 1 }), redact(2), rect(3, { x: 0, y: 0 }, { x: 1, y: 1 }));
    expect(M.exportOrder(d).map((s) => s.id)).toEqual([2, 1, 3]);
  });
});

describe('History', () => {
  it('undoes, redoes and tracks dirtiness against the saved snapshot', () => {
    const h = new M.History(doc());
    expect(h.dirty).toBe(false);
    h.commit(doc(counter(1, 1)));
    h.commit(doc(counter(1, 1), counter(2, 2)));
    expect(h.dirty).toBe(true);
    expect(h.undo()).toBe(true);
    expect(h.doc.shapes).toHaveLength(1);
    expect(h.redo()).toBe(true);
    expect(h.doc.shapes).toHaveLength(2);
    h.markSaved();
    expect(h.dirty).toBe(false);
    h.undo();
    h.commit(doc());
    expect(h.canRedo).toBe(false);
    expect(new M.History(doc()).undo()).toBe(false);
  });
});

describe('zoom and crop', () => {
  it('fitZoom never upscales', () => {
    expect(M.fitZoom(1000, 500, 500, 500)).toBe(0.5);
    expect(M.fitZoom(100, 100, 500, 500)).toBe(1);
  });
  it('zoomStep walks the preset ladder', () => {
    expect(M.zoomStep(1, 1)).toBe(1.25);
    expect(M.zoomStep(0.8, -1)).toBe(0.75);
    expect(M.zoomStep(4, 1)).toBe(4);
  });
  it('fitRatio centres the largest rect of that ratio', () => {
    expect(M.fitRatio({ x: 0, y: 0, w: 200, h: 100 }, 1)).toEqual({ x: 50, y: 0, w: 100, h: 100 });
  });
  it('cropFromDrag constrains to the ratio and to the image', () => {
    expect(M.cropFromDrag({ x: 0, y: 0 }, { x: 160, y: 200 }, 16 / 9, 1000, 1000)).toEqual({ x: 0, y: 0, w: 160, h: 90 });
    expect(M.cropFromDrag({ x: 10, y: 10 }, { x: 2000, y: 50 }, null, 100, 100)).toEqual({ x: 10, y: 10, w: 90, h: 40 });
  });
  it('viewToImage undoes zoom and crop offset', () => {
    expect(M.viewToImage({ x: 50, y: 20 }, 0.5, { x: 100, y: 10, w: 1, h: 1 })).toEqual({ x: 200, y: 50 });
  });
});
```

`src/editor/model.ts` (stubs; every function exported, bodies wrong on purpose):

```ts
export type Pt = { x: number; y: number };
export type Rect = { x: number; y: number; w: number; h: number };
export type Width = 0 | 1 | 2;
export type TextSize = 'S' | 'M' | 'L' | 'XL';
export type TextStyle = 'filled' | 'plain' | 'outline';
export type Ends = 'one' | 'both' | 'none';
export type RedactMode = 'pixelate' | 'blur' | 'solid';
export type HandleId = 'a' | 'b' | 'nw' | 'ne' | 'sw' | 'se';
export type Ratio = 'free' | '16:9' | '4:3' | '1:1';
type Base = { id: number };
type TwoPoint = Base & { a: Pt; b: Pt };
export type Shape =
  | (TwoPoint & { kind: 'arrow'; color: string; width: Width; ends: Ends; shadow: boolean })
  | (TwoPoint & { kind: 'line' | 'rect' | 'ellipse'; color: string; width: Width })
  | (TwoPoint & { kind: 'redact'; mode: RedactMode; strength: number })
  | (Base & { kind: 'pen' | 'highlight'; pts: Pt[]; color: string; width: Width })
  | (Base & { kind: 'text'; at: Pt; text: string; color: string; size: TextSize; style: TextStyle; w: number; h: number })
  | (Base & { kind: 'counter'; at: Pt; n: number; color: string });
export type Doc = { shapes: Shape[]; crop: Rect | null };

export const WIDTHS = [3, 5, 8] as const;
export const HIGHLIGHT_WIDTHS = [14, 22, 34] as const;
export const TEXT_PX: Record<TextSize, number> = { S: 14, M: 18, L: 26, XL: 36 };
export const COUNTER_R = 13;
export const SWATCHES = ['#ff453a', '#ff9f0a', '#ffd60a', '#30d158', '#0a84ff', '#bf5af2', '#ffffff', '#1c1c1e'];
export const ZOOMS = [0.1, 0.25, 0.33, 0.5, 0.67, 0.75, 1, 1.25, 1.5, 2, 3, 4];
export const RATIOS: Record<Exclude<Ratio, 'free'>, number> = { '16:9': 16 / 9, '4:3': 4 / 3, '1:1': 1 };

const ZERO: Rect = { x: 0, y: 0, w: 0, h: 0 };
export const unitFor = (_w: number, _h: number) => 0;
export const norm = (_a: Pt, _b: Pt): Rect => ZERO;
export const strokeWidth = (_s: Shape, _u: number) => 0;
export const bounds = (_s: Shape, _u = 1): Rect => ZERO;
export const distToSeg = (_p: Pt, _a: Pt, _b: Pt) => -1;
export const hitShape = (_s: Shape, _p: Pt, _tol: number, _u = 1) => false;
export const hitTest = (_d: Doc, _p: Pt, _tol: number, _u = 1): Shape | null => null;
export const translate = (s: Shape, _dx: number, _dy: number): Shape => s;
export const handles = (_s: Shape): { id: HandleId; p: Pt }[] => [];
export const dragHandle = (s: Shape, _h: HandleId, _p: Pt): Shape => s;
export const snap45 = (a: Pt, _b: Pt): Pt => a;
export const squareEnd = (a: Pt, _b: Pt): Pt => a;
export const nextCounter = (_d: Doc) => 0;
export const exportOrder = (d: Doc): Shape[] => d.shapes;
export const fitZoom = (_w: number, _h: number, _vw: number, _vh: number) => 0;
export const zoomStep = (z: number, _dir: 1 | -1) => z;
export const fitRatio = (r: Rect, _ratio: number): Rect => r;
export const cropFromDrag = (_a: Pt, _b: Pt, _ratio: number | null, _W: number, _H: number): Rect => ZERO;
export const viewToImage = (p: Pt, _zoom: number, _crop: Rect | null): Pt => p;
export class History {
  constructor(public doc: Doc) {}
  commit(_next: Doc): void {}
  undo(): boolean { return false; }
  redo(): boolean { return false; }
  markSaved(): void {}
  get canUndo() { return false; }
  get canRedo() { return false; }
  get dirty() { return false; }
}
```

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `npm test -- src/editor`
Expected: FAIL in every `describe` block.

- [ ] **Step 3: Implement `model.ts`**

Keep the type declarations and constants from Step 1 at the top of the file. Replace everything from `const ZERO` down with:

```ts
/** Annotation scale: 1 at 1080p, up to 2 on 4K-class captures. */
export function unitFor(w: number, h: number): number {
  return Math.min(2, Math.max(1, Math.min(w, h) / 1080));
}

export function norm(a: Pt, b: Pt): Rect {
  return { x: Math.min(a.x, b.x), y: Math.min(a.y, b.y), w: Math.abs(b.x - a.x), h: Math.abs(b.y - a.y) };
}

export function strokeWidth(s: Shape, u: number): number {
  if (s.kind === 'highlight') return HIGHLIGHT_WIDTHS[s.width] * u;
  return 'width' in s ? WIDTHS[s.width] * u : 0;
}

export function bounds(s: Shape, u = 1): Rect {
  switch (s.kind) {
    case 'pen':
    case 'highlight': {
      const xs = s.pts.map((p) => p.x);
      const ys = s.pts.map((p) => p.y);
      const x = Math.min(...xs);
      const y = Math.min(...ys);
      return { x, y, w: Math.max(...xs) - x, h: Math.max(...ys) - y };
    }
    case 'text':
      return { x: s.at.x, y: s.at.y, w: s.w, h: s.h };
    case 'counter': {
      const r = COUNTER_R * u;
      return { x: s.at.x - r, y: s.at.y - r, w: 2 * r, h: 2 * r };
    }
    default:
      return norm(s.a, s.b);
  }
}

export function distToSeg(p: Pt, a: Pt, b: Pt): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const len2 = dx * dx + dy * dy;
  const t = len2 ? Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / len2)) : 0;
  return Math.hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy));
}

const inside = (r: Rect, p: Pt, pad: number) =>
  p.x >= r.x - pad && p.x <= r.x + r.w + pad && p.y >= r.y - pad && p.y <= r.y + r.h + pad;

/** Strokes hit near their line; rectangles and ellipses on their outline; the rest inside their box. */
export function hitShape(s: Shape, p: Pt, tol: number, u = 1): boolean {
  const reach = tol + strokeWidth(s, u) / 2;
  switch (s.kind) {
    case 'arrow':
    case 'line':
      return distToSeg(p, s.a, s.b) <= reach;
    case 'pen':
    case 'highlight':
      return s.pts.some((q, i) => distToSeg(p, s.pts[Math.max(0, i - 1)], q) <= reach);
    case 'rect': {
      const r = norm(s.a, s.b);
      const edge = Math.min(Math.abs(p.x - r.x), Math.abs(p.x - r.x - r.w), Math.abs(p.y - r.y), Math.abs(p.y - r.y - r.h));
      return inside(r, p, reach) && edge <= reach;
    }
    case 'ellipse': {
      const r = norm(s.a, s.b);
      const rx = r.w / 2;
      const ry = r.h / 2;
      if (!rx || !ry) return false;
      const d = Math.hypot((p.x - r.x - rx) / rx, (p.y - r.y - ry) / ry);
      return Math.abs(d - 1) * Math.min(rx, ry) <= reach;
    }
    default:
      return inside(bounds(s, u), p, tol);
  }
}

export function hitTest(doc: Doc, p: Pt, tol: number, u = 1): Shape | null {
  for (let i = doc.shapes.length - 1; i >= 0; i--) if (hitShape(doc.shapes[i], p, tol, u)) return doc.shapes[i];
  return null;
}

export function translate(s: Shape, dx: number, dy: number): Shape {
  const m = (p: Pt) => ({ x: p.x + dx, y: p.y + dy });
  switch (s.kind) {
    case 'pen':
    case 'highlight':
      return { ...s, pts: s.pts.map(m) };
    case 'text':
    case 'counter':
      return { ...s, at: m(s.at) };
    default:
      return { ...s, a: m(s.a), b: m(s.b) };
  }
}

export function handles(s: Shape): { id: HandleId; p: Pt }[] {
  if (s.kind === 'arrow' || s.kind === 'line') return [{ id: 'a', p: s.a }, { id: 'b', p: s.b }];
  if (s.kind === 'rect' || s.kind === 'ellipse' || s.kind === 'redact') {
    const r = norm(s.a, s.b);
    return [
      { id: 'nw', p: { x: r.x, y: r.y } },
      { id: 'ne', p: { x: r.x + r.w, y: r.y } },
      { id: 'sw', p: { x: r.x, y: r.y + r.h } },
      { id: 'se', p: { x: r.x + r.w, y: r.y + r.h } },
    ];
  }
  return [];
}

export function dragHandle(s: Shape, h: HandleId, p: Pt): Shape {
  if (s.kind === 'arrow' || s.kind === 'line') return h === 'a' ? { ...s, a: p } : { ...s, b: p };
  if (s.kind === 'rect' || s.kind === 'ellipse' || s.kind === 'redact') {
    const r = norm(s.a, s.b);
    const opposite: Partial<Record<HandleId, Pt>> = {
      nw: { x: r.x + r.w, y: r.y + r.h },
      ne: { x: r.x, y: r.y + r.h },
      sw: { x: r.x + r.w, y: r.y },
      se: { x: r.x, y: r.y },
    };
    const fixed = opposite[h];
    return fixed ? { ...s, a: fixed, b: p } : s;
  }
  return s;
}

/** End point snapped to the nearest 45° (Shift on arrows and lines). */
export function snap45(a: Pt, b: Pt): Pt {
  const step = Math.PI / 4;
  const ang = Math.round(Math.atan2(b.y - a.y, b.x - a.x) / step) * step;
  const len = Math.hypot(b.x - a.x, b.y - a.y);
  return { x: a.x + Math.round(Math.cos(ang) * len), y: a.y + Math.round(Math.sin(ang) * len) };
}

/** End point giving a square/circle (Shift on rectangles, ellipses, redactions). */
export function squareEnd(a: Pt, b: Pt): Pt {
  const s = Math.max(Math.abs(b.x - a.x), Math.abs(b.y - a.y));
  return { x: a.x + Math.sign(b.x - a.x || 1) * s, y: a.y + Math.sign(b.y - a.y || 1) * s };
}

export function nextCounter(doc: Doc): number {
  return doc.shapes.reduce((n, s) => (s.kind === 'counter' ? Math.max(n, s.n) : n), 0) + 1;
}

/** Redactions read the original pixels, so they paint first; the rest keep z-order. */
export function exportOrder(doc: Doc): Shape[] {
  return [...doc.shapes.filter((s) => s.kind === 'redact'), ...doc.shapes.filter((s) => s.kind !== 'redact')];
}

export function fitZoom(w: number, h: number, vw: number, vh: number): number {
  return Math.min(1, vw / w, vh / h);
}

export function zoomStep(z: number, dir: 1 | -1): number {
  if (dir > 0) return ZOOMS.find((s) => s > z + 1e-6) ?? ZOOMS[ZOOMS.length - 1];
  return [...ZOOMS].reverse().find((s) => s < z - 1e-6) ?? ZOOMS[0];
}

/** Largest rect of `ratio` centred in `r`. */
export function fitRatio(r: Rect, ratio: number): Rect {
  const w = Math.min(r.w, r.h * ratio);
  const h = w / ratio;
  return { x: r.x + (r.w - w) / 2, y: r.y + (r.h - h) / 2, w, h };
}

/** Crop rect dragged from `a` to `b`, clipped to the image and held to `ratio` when set. */
export function cropFromDrag(a: Pt, b: Pt, ratio: number | null, W: number, H: number): Rect {
  const bx = Math.max(0, Math.min(W, b.x));
  const by = Math.max(0, Math.min(H, b.y));
  if (!ratio) return norm(a, { x: bx, y: by });
  let w = Math.abs(bx - a.x);
  let h = Math.abs(by - a.y);
  if (w / ratio > h) w = h * ratio;
  else h = w / ratio;
  return norm(a, { x: a.x + Math.sign(bx - a.x || 1) * w, y: a.y + Math.sign(by - a.y || 1) * h });
}

export function viewToImage(p: Pt, zoom: number, crop: Rect | null): Pt {
  return { x: p.x / zoom + (crop?.x ?? 0), y: p.y / zoom + (crop?.y ?? 0) };
}

/** Undo/redo over immutable Doc snapshots; `dirty` compares against the last saved one. */
export class History {
  private past: Doc[] = [];
  private future: Doc[] = [];
  private saved: Doc;

  constructor(public doc: Doc) {
    this.saved = doc;
  }

  commit(next: Doc): void {
    this.past.push(this.doc);
    this.future = [];
    this.doc = next;
  }

  undo(): boolean {
    const prev = this.past.pop();
    if (!prev) return false;
    this.future.push(this.doc);
    this.doc = prev;
    return true;
  }

  redo(): boolean {
    const next = this.future.pop();
    if (!next) return false;
    this.past.push(this.doc);
    this.doc = next;
    return true;
  }

  markSaved(): void {
    this.saved = this.doc;
  }

  get canUndo() {
    return this.past.length > 0;
  }

  get canRedo() {
    return this.future.length > 0;
  }

  get dirty() {
    return this.doc !== this.saved;
  }
}
```

- [ ] **Step 4: Run the tests**

Run: `npm test`
Expected: every suite passes (Plan 1's plus 17 new tests), with no warnings.

- [ ] **Step 5: Commit**

```bash
git add src/editor/model.ts src/editor/model.test.ts && git commit -m "feat(editor): pure document model with hit testing, handles, history, zoom and crop maths

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 3: Canvas rendering and export (`render.ts`) + style popover (`styles.ts`)

**Files:**
- Create: `src/editor/render.ts`, `src/editor/styles.ts`

**Interfaces:**
- Consumes: everything in `model.ts`.
- Produces:
  - `render.ts`: `ink(color)`, `measureText(ctx, text, px, style) -> {w, h}`, `drawShape(ctx, src, s, u)`, `drawDoc(ctx, src, doc, u, skipId?)`, `drawSelection(ctx, s, zoom, u)`, `drawCrop(ctx, r, W, H, zoom)`, `exportPng(src, doc, u, W, H) -> Promise<Uint8Array>`.
  - `styles.ts`: `type Tool`, `SECTIONS`, the mutable `style` object, `currentColor(tool)`, `onStyleChange(fn)`, `openPopover(tool, anchor)`, `closePopover()`, `popoverOpen()`.

- [ ] **Step 1: Write `src/editor/render.ts`**

```ts
import { type Doc, type Rect, type Shape, COUNTER_R, TEXT_PX, bounds, exportOrder, handles, norm, strokeWidth } from './model';

export type Src = CanvasImageSource & { width: number; height: number };

const LIGHT = new Set(['#ffffff', '#ffd60a']);
/** Readable text colour on top of `color` (dark on white/yellow, white otherwise). */
export const ink = (color: string) => (LIGHT.has(color.toLowerCase()) ? '#1c1c1e' : '#ffffff');
const font = (px: number, weight = 600) => `${weight} ${px}px Inter, system-ui, sans-serif`;

/** Box a text annotation occupies (stored on the shape when the text is committed). */
export function measureText(ctx: CanvasRenderingContext2D, text: string, px: number, style: string): { w: number; h: number } {
  ctx.save();
  ctx.font = font(px);
  const w = Math.max(...text.split('\n').map((l) => ctx.measureText(l).width));
  ctx.restore();
  const padX = style === 'filled' ? px * 0.6 : 0;
  const padY = style === 'filled' ? px * 0.35 : 0;
  return { w: Math.ceil(w + padX * 2), h: Math.ceil(text.split('\n').length * px * 1.25 + padY * 2) };
}

function seg(ctx: CanvasRenderingContext2D, a: { x: number; y: number }, b: { x: number; y: number }) {
  ctx.beginPath();
  ctx.moveTo(a.x, a.y);
  ctx.lineTo(b.x, b.y);
  ctx.stroke();
}

function head(ctx: CanvasRenderingContext2D, tip: { x: number; y: number }, ang: number, size: number) {
  ctx.beginPath();
  ctx.moveTo(tip.x, tip.y);
  ctx.lineTo(tip.x - size * Math.cos(ang - 0.45), tip.y - size * Math.sin(ang - 0.45));
  ctx.lineTo(tip.x - size * Math.cos(ang + 0.45), tip.y - size * Math.sin(ang + 0.45));
  ctx.closePath();
  ctx.fill();
}

function arrow(ctx: CanvasRenderingContext2D, s: Extract<Shape, { kind: 'arrow' }>, u: number) {
  const w = strokeWidth(s, u);
  const size = Math.max(10 * u, w * 3.2);
  const ang = Math.atan2(s.b.y - s.a.y, s.b.x - s.a.x);
  const back = { x: Math.cos(ang) * size * 0.6, y: Math.sin(ang) * size * 0.6 };
  if (s.shadow) {
    ctx.shadowColor = 'rgba(0,0,0,.35)';
    ctx.shadowBlur = 4 * u;
    ctx.shadowOffsetY = 2 * u;
  }
  ctx.strokeStyle = ctx.fillStyle = s.color;
  ctx.lineWidth = w;
  // Shorten the shaft so its round cap doesn't poke through the head.
  const a = s.ends === 'both' ? { x: s.a.x + back.x, y: s.a.y + back.y } : s.a;
  const b = s.ends === 'none' ? s.b : { x: s.b.x - back.x, y: s.b.y - back.y };
  seg(ctx, a, b);
  if (s.ends !== 'none') head(ctx, s.b, ang, size);
  if (s.ends === 'both') head(ctx, s.a, ang + Math.PI, size);
}

function text(ctx: CanvasRenderingContext2D, s: Extract<Shape, { kind: 'text' }>, u: number) {
  const px = TEXT_PX[s.size] * u;
  const padX = s.style === 'filled' ? px * 0.6 : 0;
  const padY = s.style === 'filled' ? px * 0.35 : 0;
  ctx.font = font(px);
  ctx.textBaseline = 'top';
  if (s.style === 'filled') {
    ctx.fillStyle = s.color;
    ctx.beginPath();
    ctx.roundRect(s.at.x, s.at.y, s.w, s.h, px * 0.4);
    ctx.fill();
  }
  s.text.split('\n').forEach((line, i) => {
    const x = s.at.x + padX;
    const y = s.at.y + padY + i * px * 1.25 + px * 0.125;
    if (s.style === 'outline') {
      ctx.lineWidth = px * 0.22;
      ctx.strokeStyle = s.color;
      ctx.strokeText(line, x, y);
    }
    ctx.fillStyle = s.style === 'plain' ? s.color : ink(s.color);
    ctx.fillText(line, x, y);
  });
}

function counter(ctx: CanvasRenderingContext2D, s: Extract<Shape, { kind: 'counter' }>, u: number) {
  const r = COUNTER_R * u;
  ctx.shadowColor = 'rgba(0,0,0,.3)';
  ctx.shadowBlur = 4 * u;
  ctx.shadowOffsetY = 1.5 * u;
  ctx.fillStyle = s.color;
  ctx.beginPath();
  ctx.arc(s.at.x, s.at.y, r, 0, Math.PI * 2);
  ctx.fill();
  ctx.shadowColor = 'transparent';
  ctx.lineWidth = 2 * u;
  ctx.strokeStyle = '#ffffff';
  ctx.stroke();
  ctx.fillStyle = ink(s.color);
  ctx.font = font(Math.round(r * 1.05), 700);
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(String(s.n), s.at.x, s.at.y + u);
}

/** Pixelate/blur re-sample the ORIGINAL pixels through a small canvas, so the saved PNG holds no trace of them. */
function redact(ctx: CanvasRenderingContext2D, src: Src, r: Rect, mode: string, strength: number) {
  if (r.w < 1 || r.h < 1) return;
  if (mode === 'solid') {
    ctx.fillStyle = '#1c1c1e';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    return;
  }
  const block = Math.round(mode === 'pixelate' ? 6 + strength * 26 : 4 + strength * 16);
  const small = document.createElement('canvas');
  small.width = Math.max(1, Math.round(r.w / block));
  small.height = Math.max(1, Math.round(r.h / block));
  const sc = small.getContext('2d')!;
  sc.imageSmoothingEnabled = true;
  sc.drawImage(src, r.x, r.y, r.w, r.h, 0, 0, small.width, small.height);
  ctx.imageSmoothingEnabled = mode === 'blur';
  ctx.imageSmoothingQuality = 'high';
  ctx.drawImage(small, 0, 0, small.width, small.height, r.x, r.y, r.w, r.h);
}

export function drawShape(ctx: CanvasRenderingContext2D, src: Src, s: Shape, u: number) {
  ctx.save();
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  switch (s.kind) {
    case 'redact':
      redact(ctx, src, norm(s.a, s.b), s.mode, s.strength);
      break;
    case 'arrow':
      arrow(ctx, s, u);
      break;
    case 'line':
      ctx.strokeStyle = s.color;
      ctx.lineWidth = strokeWidth(s, u);
      seg(ctx, s.a, s.b);
      break;
    case 'rect': {
      const r = norm(s.a, s.b);
      ctx.strokeStyle = s.color;
      ctx.lineWidth = strokeWidth(s, u);
      ctx.beginPath();
      ctx.roundRect(r.x, r.y, r.w, r.h, 6 * u);
      ctx.stroke();
      break;
    }
    case 'ellipse': {
      const r = norm(s.a, s.b);
      ctx.strokeStyle = s.color;
      ctx.lineWidth = strokeWidth(s, u);
      ctx.beginPath();
      ctx.ellipse(r.x + r.w / 2, r.y + r.h / 2, r.w / 2, r.h / 2, 0, 0, Math.PI * 2);
      ctx.stroke();
      break;
    }
    case 'pen':
    case 'highlight': {
      if (s.kind === 'highlight') ctx.globalAlpha = 0.4;
      ctx.strokeStyle = s.color;
      ctx.lineWidth = strokeWidth(s, u);
      ctx.beginPath();
      s.pts.forEach((p, i) => (i ? ctx.lineTo(p.x, p.y) : ctx.moveTo(p.x, p.y)));
      if (s.pts.length === 1) ctx.lineTo(s.pts[0].x + 0.01, s.pts[0].y);
      ctx.stroke();
      break;
    }
    case 'text':
      text(ctx, s, u);
      break;
    case 'counter':
      counter(ctx, s, u);
      break;
  }
  ctx.restore();
}

/** The capture plus its annotations in export order; `skipId` hides a text being edited. */
export function drawDoc(ctx: CanvasRenderingContext2D, src: Src, doc: Doc, u: number, skipId?: number) {
  ctx.drawImage(src, 0, 0);
  for (const s of exportOrder(doc)) if (s.id !== skipId) drawShape(ctx, src, s, u);
}

export function drawSelection(ctx: CanvasRenderingContext2D, s: Shape, zoom: number, u: number) {
  ctx.save();
  const hs = handles(s);
  if (!hs.length) {
    const b = bounds(s, u);
    const pad = 4 / zoom;
    ctx.setLineDash([4 / zoom, 3 / zoom]);
    ctx.strokeStyle = '#0a84ff';
    ctx.lineWidth = 1.5 / zoom;
    ctx.strokeRect(b.x - pad, b.y - pad, b.w + 2 * pad, b.h + 2 * pad);
  }
  for (const h of hs) {
    ctx.beginPath();
    ctx.arc(h.p.x, h.p.y, 5.5 / zoom, 0, Math.PI * 2);
    ctx.fillStyle = '#ffffff';
    ctx.fill();
    ctx.lineWidth = 2 / zoom;
    ctx.strokeStyle = '#0a84ff';
    ctx.stroke();
  }
  ctx.restore();
}

/** Crop mode: dim outside `r`, rule-of-thirds grid, corner brackets. */
export function drawCrop(ctx: CanvasRenderingContext2D, r: Rect, W: number, H: number, zoom: number) {
  ctx.save();
  ctx.fillStyle = 'rgba(0,0,0,.6)';
  ctx.beginPath();
  ctx.rect(0, 0, W, H);
  ctx.rect(r.x, r.y, r.w, r.h);
  ctx.fill('evenodd');
  ctx.strokeStyle = 'rgba(255,255,255,.4)';
  ctx.lineWidth = 1 / zoom;
  for (const t of [1 / 3, 2 / 3]) {
    seg(ctx, { x: r.x + r.w * t, y: r.y }, { x: r.x + r.w * t, y: r.y + r.h });
    seg(ctx, { x: r.x, y: r.y + r.h * t }, { x: r.x + r.w, y: r.y + r.h * t });
  }
  ctx.strokeStyle = '#ffffff';
  ctx.lineWidth = 1.5 / zoom;
  ctx.strokeRect(r.x, r.y, r.w, r.h);
  const L = 16 / zoom;
  ctx.lineWidth = 3 / zoom;
  ctx.lineCap = 'square';
  for (const [x, y, dx, dy] of [
    [r.x, r.y, 1, 1],
    [r.x + r.w, r.y, -1, 1],
    [r.x, r.y + r.h, 1, -1],
    [r.x + r.w, r.y + r.h, -1, -1],
  ]) {
    ctx.beginPath();
    ctx.moveTo(x + dx * L, y);
    ctx.lineTo(x, y);
    ctx.lineTo(x, y + dy * L);
    ctx.stroke();
  }
  ctx.restore();
}

/** Flattens `doc` over the original capture (cropped) into PNG bytes. */
export async function exportPng(src: Src, doc: Doc, u: number, W: number, H: number): Promise<Uint8Array> {
  const r = doc.crop ?? { x: 0, y: 0, w: W, h: H };
  const c = document.createElement('canvas');
  c.width = Math.round(r.w);
  c.height = Math.round(r.h);
  const ctx = c.getContext('2d')!;
  ctx.translate(-Math.round(r.x), -Math.round(r.y));
  drawDoc(ctx, src, doc, u);
  const blob = await new Promise<Blob>((ok, no) => c.toBlob((b) => (b ? ok(b) : no(new Error('PNG encoding failed'))), 'image/png'));
  return new Uint8Array(await blob.arrayBuffer());
}
```

- [ ] **Step 2: Write `src/editor/styles.ts`**

```ts
import * as M from './model';

export type Tool = 'select' | 'crop' | 'arrow' | 'rect' | 'ellipse' | 'line' | 'pen' | 'highlight' | 'text' | 'counter' | 'redact';
type Section = 'color' | 'width' | 'ends' | 'shadow' | 'size' | 'textStyle' | 'mode' | 'strength';

export const SECTIONS: Record<Tool, Section[]> = {
  select: [],
  crop: [],
  arrow: ['color', 'width', 'ends', 'shadow'],
  rect: ['color', 'width'],
  ellipse: ['color', 'width'],
  line: ['color', 'width'],
  pen: ['color', 'width'],
  highlight: ['color', 'width'],
  text: ['color', 'size', 'textStyle'],
  counter: ['color'],
  redact: ['mode', 'strength'],
};

/** The style new shapes get; the popover edits it, the selected shape adopts it. */
export const style = {
  color: '#ff453a',
  highlight: '#ffd60a',
  width: 1 as M.Width,
  ends: 'one' as M.Ends,
  shadow: true,
  size: 'M' as M.TextSize,
  textStyle: 'filled' as M.TextStyle,
  mode: 'pixelate' as M.RedactMode,
  strength: 0.6,
};

export const currentColor = (t: Tool) => (t === 'highlight' ? style.highlight : style.color);

const pop = document.querySelector<HTMLElement>('#pop')!;
let forTool: Tool = 'arrow';
let listener: () => void = () => {};

export const onStyleChange = (fn: () => void) => {
  listener = fn;
};
export const popoverOpen = () => !pop.hidden;
export const closePopover = () => {
  pop.hidden = true;
};

const cap = (s: string) => s[0].toUpperCase() + s.slice(1);

function section(s: Section): string {
  switch (s) {
    case 'color':
      return `<div class="lbl">Color</div><div class="sws">${M.SWATCHES.map(
        (c) => `<button data-color="${c}" style="background:${c}" aria-label="Color ${c}"></button>`,
      ).join('')}</div>`;
    case 'width':
      return `<div class="lbl">Width</div><div class="seg">${[0, 1, 2]
        .map((w) => `<button data-width="${w}" aria-label="Width ${'SML'[w]}"><b style="height:${[2, 4, 7][w]}px"></b></button>`)
        .join('')}</div>`;
    case 'ends':
      return `<div class="lbl">Ends</div><div class="seg">${(['one', 'both', 'none'] as const)
        .map((e) => `<button data-ends="${e}" aria-label="Arrowheads: ${e}"><svg class="ic sm" aria-hidden="true"><use href="#e-${e === 'both' ? 'two' : e}"/></svg></button>`)
        .join('')}</div>`;
    case 'shadow':
      return `<label class="tog">Drop shadow<button class="switch" role="switch" data-shadow="1"></button></label>`;
    case 'size':
      return `<div class="lbl">Size</div><div class="seg">${['S', 'M', 'L', 'XL'].map((z) => `<button data-size="${z}">${z}</button>`).join('')}</div>`;
    case 'textStyle':
      return `<div class="lbl">Style</div><div class="seg">${['filled', 'plain', 'outline'].map((v) => `<button data-tstyle="${v}">${cap(v)}</button>`).join('')}</div>`;
    case 'mode':
      return `<div class="lbl">Mode</div><div class="seg">${['pixelate', 'blur', 'solid'].map((v) => `<button data-rmode="${v}">${cap(v)}</button>`).join('')}</div>`;
    case 'strength':
      return `<div class="lbl">Strength</div><input type="range" min="0" max="1" step="0.05" data-strength="1" aria-label="Strength" />
        <p class="note">Drag over any area. The pixels are replaced in the saved file, so what's underneath can't be recovered.</p>`;
  }
}

function sync() {
  const on = (sel: string, match: (el: HTMLElement) => boolean) =>
    pop.querySelectorAll<HTMLElement>(sel).forEach((el) => el.classList.toggle('on', match(el)));
  on('[data-color]', (el) => el.dataset.color === currentColor(forTool));
  on('[data-width]', (el) => Number(el.dataset.width) === style.width);
  on('[data-ends]', (el) => el.dataset.ends === style.ends);
  on('[data-shadow]', () => style.shadow);
  on('[data-size]', (el) => el.dataset.size === style.size);
  on('[data-tstyle]', (el) => el.dataset.tstyle === style.textStyle);
  on('[data-rmode]', (el) => el.dataset.rmode === style.mode);
  pop.querySelector('[data-shadow]')?.setAttribute('aria-checked', String(style.shadow));
  const range = pop.querySelector<HTMLInputElement>('[data-strength]');
  if (range) range.value = String(style.strength);
}

/** Opens the style popover for `tool` next to `anchor` (a rail button). */
export function openPopover(tool: Tool, anchor: HTMLElement) {
  const sections = SECTIONS[tool];
  if (!sections.length) return;
  forTool = tool;
  pop.innerHTML = sections.map(section).join('');
  sync();
  pop.hidden = false;
  const host = (pop.offsetParent as HTMLElement).getBoundingClientRect();
  const a = anchor.getBoundingClientRect();
  const top = Math.min(a.top - host.top - 10, host.height - pop.offsetHeight - 8);
  pop.style.top = `${Math.max(8, top)}px`;
}

pop.addEventListener('click', (e) => {
  const b = (e.target as Element).closest<HTMLElement>('button');
  if (!b) return;
  const d = b.dataset;
  if (d.color) {
    if (forTool === 'highlight') style.highlight = d.color;
    else style.color = d.color;
  } else if (d.width) style.width = Number(d.width) as M.Width;
  else if (d.ends) style.ends = d.ends as M.Ends;
  else if (d.shadow) style.shadow = !style.shadow;
  else if (d.size) style.size = d.size as M.TextSize;
  else if (d.tstyle) style.textStyle = d.tstyle as M.TextStyle;
  else if (d.rmode) style.mode = d.rmode as M.RedactMode;
  else return;
  sync();
  listener();
});

// `change` (not `input`) so dragging the slider makes one undo step, not dozens.
pop.addEventListener('change', (e) => {
  const t = e.target as HTMLInputElement;
  if (!t.dataset.strength) return;
  style.strength = Number(t.value);
  listener();
});
```

- [ ] **Step 3: Type-check and build**

Run: `npm run build && npm test`
Expected: `tsc` reports no errors (both modules are unused until Task 4, but they must compile), and the tests still pass.

- [ ] **Step 4: Commit**

```bash
git add src/editor/render.ts src/editor/styles.ts && git commit -m "feat(editor): canvas rendering, redaction, export and tool style popover

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

### Task 4: Editor interaction (`main.ts`): tools, selection, text, crop, zoom, save/close flows

**Files:**
- Replace: `src/editor/main.ts`

**Interfaces:**
- Consumes:
  - `model.ts` and `render.ts` (Task 2–3 names above), `styles.ts` (`style`, `SECTIONS`, `Tool`, `currentColor`, `onStyleChange`, `openPopover`, `closePopover`, `popoverOpen`).
  - `ipc.{editorInfo, readCapture, saveImage, retryCopy, editorDelete, copyPath, revealCapture, closeWindow}`.
- Produces: the finished editor. No exports.

- [ ] **Step 1: Replace `src/editor/main.ts`**

```ts
import '../shared/base';
import '../shared/glass.css';
import './editor.css';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { mountIcons } from '../shared/icons';
import * as ipc from '../shared/ipc';
import * as M from './model';
import { drawCrop, drawDoc, drawSelection, exportPng, ink, measureText } from './render';
import { type Tool, SECTIONS, closePopover, currentColor, onStyleChange, openPopover, popoverOpen, style } from './styles';

mountIcons();

const $ = <T extends HTMLElement = HTMLElement>(s: string) => document.querySelector(s) as T;
const stage = $('#stage');
const canvas = $<HTMLCanvasElement>('#view');
const ctx = canvas.getContext('2d')!;
const texted = $<HTMLTextAreaElement>('#texted');

const info = await ipc.editorInfo();
const img = await createImageBitmap(new Blob([await ipc.readCapture(info.path)], { type: 'image/png' }));
const W = img.width;
const H = img.height;
const u = M.unitFor(W, H);
const hist = new M.History({ shapes: [], crop: null });
$('#title').textContent = info.name;
$('#path').textContent = info.display;

type Drag =
  | { kind: 'draw'; start: M.Pt; shape: M.Shape }
  | { kind: 'move'; start: M.Pt; orig: M.Shape }
  | { kind: 'handle'; handle: M.HandleId; orig: M.Shape }
  | { kind: 'crop-new'; start: M.Pt }
  | { kind: 'crop-move'; start: M.Pt; orig: M.Rect }
  | { kind: 'crop-corner'; fixed: M.Pt };

let tool: Tool = 'arrow';
let selectedId: number | null = null;
let live: M.Doc | null = null; // preview while dragging; committed on pointerup
let drag: Drag | null = null;
let zoom = 1;
let fitMode = true;
let nextId = 1;
let editingText: { id: number | null; at: M.Pt } | null = null;
let crop: { draft: M.Rect; ratio: M.Ratio } | null = null;
let busy = false;

const doc = () => live ?? hist.doc;
const selected = () => doc().shapes.find((s) => s.id === selectedId) ?? null;
const full = (): M.Rect => ({ x: 0, y: 0, w: W, h: H });
/** What the canvas shows: the whole capture while cropping, otherwise the cropped area. */
const viewRect = (): M.Rect => (crop ? full() : (doc().crop ?? full()));

// ---------- painting ----------

function paint() {
  const v = viewRect();
  const cw = Math.round(v.w);
  const ch = Math.round(v.h);
  if (canvas.width !== cw || canvas.height !== ch) {
    canvas.width = cw;
    canvas.height = ch;
  }
  canvas.style.width = `${cw * zoom}px`;
  canvas.style.height = `${ch * zoom}px`;
  ctx.setTransform(1, 0, 0, 1, -v.x, -v.y);
  ctx.clearRect(v.x, v.y, v.w, v.h);
  drawDoc(ctx, img, doc(), u, editingText?.id ?? undefined);
  const s = selected();
  if (s && !crop && !editingText) drawSelection(ctx, s, zoom, u);
  if (crop) drawCrop(ctx, crop.draft, W, H, zoom);
}

function render() {
  paint();
  document.body.dataset.tool = tool;
  $('#undo').toggleAttribute('disabled', !hist.canUndo);
  $('#redo').toggleAttribute('disabled', !hist.canRedo);
  $('#dirty').hidden = !hist.dirty;
  $('#zoom-val').textContent = `${Math.round(zoom * 100)}%`;
  const out = hist.doc.crop ?? full();
  $('#meta').textContent = `${Math.round(out.w)} × ${Math.round(out.h)} · PNG`;
  document.querySelectorAll<HTMLElement>('#rail [data-tool]').forEach((b) => b.classList.toggle('on', b.dataset.tool === tool));
  $('#swatch i').style.background = currentColor(tool);
  if (crop) $('#cropdim').textContent = `${Math.round(crop.draft.w)} × ${Math.round(crop.draft.h)}`;
  document.querySelectorAll<HTMLElement>('#ratios [data-ratio]').forEach((b) => b.classList.toggle('on', b.dataset.ratio === crop?.ratio));
}

function commit(next: M.Doc) {
  hist.commit(next);
  render();
}

function replaceShape(next: M.Shape) {
  commit({ ...hist.doc, shapes: hist.doc.shapes.map((s) => (s.id === next.id ? next : s)) });
}

// ---------- zoom ----------

function fit() {
  fitMode = true;
  const v = viewRect();
  zoom = M.fitZoom(v.w, v.h, stage.clientWidth - 140, stage.clientHeight - 60);
}

function setZoom(z: number) {
  fitMode = false;
  zoom = z;
  render();
}

addEventListener('resize', () => {
  if (fitMode) fit();
  render();
});

stage.addEventListener(
  'wheel',
  (e) => {
    if (!e.ctrlKey) return;
    e.preventDefault();
    setZoom(M.zoomStep(zoom, e.deltaY < 0 ? 1 : -1));
  },
  { passive: false },
);

// ---------- styles ----------

function adoptStyle(s: M.Shape) {
  if (s.kind === 'highlight') style.highlight = s.color;
  else if ('color' in s) style.color = s.color;
  if ('width' in s) style.width = s.width;
  if (s.kind === 'arrow') {
    style.ends = s.ends;
    style.shadow = s.shadow;
  }
  if (s.kind === 'text') {
    style.size = s.size;
    style.textStyle = s.style;
  }
  if (s.kind === 'redact') {
    style.mode = s.mode;
    style.strength = s.strength;
  }
}

function restyle(s: M.Shape): M.Shape {
  switch (s.kind) {
    case 'arrow':
      return { ...s, color: style.color, width: style.width, ends: style.ends, shadow: style.shadow };
    case 'line':
    case 'rect':
    case 'ellipse':
    case 'pen':
      return { ...s, color: style.color, width: style.width };
    case 'highlight':
      return { ...s, color: style.highlight, width: style.width };
    case 'counter':
      return { ...s, color: style.color };
    case 'redact':
      return { ...s, mode: style.mode, strength: style.strength };
    case 'text':
      return { ...s, color: style.color, size: style.size, style: style.textStyle, ...measureText(ctx, s.text, M.TEXT_PX[style.size] * u, style.textStyle) };
  }
}

onStyleChange(() => {
  const s = hist.doc.shapes.find((x) => x.id === selectedId);
  if (s) replaceShape(restyle(s));
  if (editingText) placeTextarea();
  render();
});

function select(s: M.Shape | null) {
  selectedId = s?.id ?? null;
  if (s) adoptStyle(s);
}

// ---------- tools ----------

function setTool(t: Tool) {
  if (crop && t !== 'crop') exitCrop(false);
  if (t === 'crop') {
    if (!crop) enterCrop();
    return;
  }
  if (t === tool && SECTIONS[t].length) {
    openPopover(t, $(`#rail [data-tool="${t}"]`));
    return;
  }
  tool = t;
  closePopover();
  render();
}

function newShape(t: Tool, p: M.Pt): M.Shape | null {
  const id = nextId++;
  switch (t) {
    case 'arrow':
      return { id, kind: 'arrow', a: p, b: p, color: style.color, width: style.width, ends: style.ends, shadow: style.shadow };
    case 'line':
    case 'rect':
    case 'ellipse':
      return { id, kind: t, a: p, b: p, color: style.color, width: style.width };
    case 'redact':
      return { id, kind: 'redact', a: p, b: p, mode: style.mode, strength: style.strength };
    case 'pen':
      return { id, kind: 'pen', pts: [p], color: style.color, width: style.width };
    case 'highlight':
      return { id, kind: 'highlight', pts: [p], color: style.highlight, width: style.width };
    default:
      return null;
  }
}

function toImage(e: MouseEvent): M.Pt {
  const r = canvas.getBoundingClientRect();
  return M.viewToImage({ x: e.clientX - r.left, y: e.clientY - r.top }, zoom, viewRect());
}

// ---------- pointer ----------

canvas.addEventListener('pointerdown', (e) => {
  if (e.button !== 0) return;
  closePopover();
  if (editingText) return commitText();
  canvas.setPointerCapture(e.pointerId);
  const p = toImage(e);
  if (crop) return startCropDrag(p);
  const sel = selected();
  const handle = sel && M.handles(sel).find((h) => Math.hypot(h.p.x - p.x, h.p.y - p.y) <= 9 / zoom);
  if (sel && handle) {
    drag = { kind: 'handle', handle: handle.id, orig: sel };
    return;
  }
  if (tool === 'select') {
    const hit = M.hitTest(hist.doc, p, 6 / zoom, u);
    select(hit);
    if (hit) drag = { kind: 'move', start: p, orig: hit };
    return render();
  }
  if (tool === 'text') {
    const hit = M.hitTest(hist.doc, p, 4 / zoom, u);
    return startText(hit?.kind === 'text' ? hit : null, p);
  }
  if (tool === 'counter') {
    const id = nextId++;
    commit({ ...hist.doc, shapes: [...hist.doc.shapes, { id, kind: 'counter', at: p, n: M.nextCounter(hist.doc), color: style.color }] });
    selectedId = id;
    return render();
  }
  const shape = newShape(tool, p);
  if (!shape) return;
  drag = { kind: 'draw', start: p, shape };
  live = { ...hist.doc, shapes: [...hist.doc.shapes, shape] };
  paint();
});

canvas.addEventListener('pointermove', (e) => {
  if (!drag) return;
  const p = toImage(e);
  if (drag.kind === 'crop-new' || drag.kind === 'crop-move' || drag.kind === 'crop-corner') return moveCropDrag(p);
  const base = hist.doc;
  let next: M.Shape;
  if (drag.kind === 'draw') {
    const s = drag.shape;
    if (s.kind === 'pen' || s.kind === 'highlight') {
      s.pts.push(p); // the draft isn't committed yet, so mutating it is safe
      next = s;
    } else if (s.kind === 'text' || s.kind === 'counter') return;
    else {
      const b = !e.shiftKey ? p : s.kind === 'arrow' || s.kind === 'line' ? M.snap45(drag.start, p) : M.squareEnd(drag.start, p);
      next = { ...s, b };
      drag.shape = next;
    }
    live = { ...base, shapes: [...base.shapes.filter((x) => x.id !== next.id), next] };
  } else {
    next = drag.kind === 'move' ? M.translate(drag.orig, p.x - drag.start.x, p.y - drag.start.y) : M.dragHandle(drag.orig, drag.handle, p);
    live = { ...base, shapes: base.shapes.map((x) => (x.id === next.id ? next : x)) };
  }
  paint();
});

canvas.addEventListener('pointerup', () => {
  const d = drag;
  drag = null;
  if (!d) return;
  if (d.kind === 'crop-new' || d.kind === 'crop-move' || d.kind === 'crop-corner') return render();
  const next = live;
  live = null;
  if (!next) return render();
  if (d.kind === 'draw') {
    const s = next.shapes[next.shapes.length - 1];
    const b = M.bounds(s, u);
    const tiny = s.kind === 'pen' || s.kind === 'highlight' ? s.pts.length < 2 : Math.max(b.w, b.h) < 3;
    if (tiny) return render();
    selectedId = s.id;
  }
  commit(next);
});

canvas.addEventListener('dblclick', (e) => {
  const hit = M.hitTest(hist.doc, toImage(e), 4 / zoom, u);
  if (hit?.kind === 'text') startText(hit, hit.at);
});

// ---------- text ----------

function placeTextarea() {
  if (!editingText) return;
  const v = viewRect();
  const px = M.TEXT_PX[style.size] * u * zoom;
  texted.dataset.style = style.textStyle;
  texted.style.setProperty('--c', style.color);
  texted.style.setProperty('--ink', ink(style.color));
  Object.assign(texted.style, {
    left: `${canvas.offsetLeft + (editingText.at.x - v.x) * zoom}px`,
    top: `${canvas.offsetTop + (editingText.at.y - v.y) * zoom}px`,
    fontSize: `${px}px`,
    fontFamily: 'Inter, system-ui, sans-serif',
  });
  autosize();
}

function autosize() {
  const m = measureText(ctx, texted.value || ' ', M.TEXT_PX[style.size] * u, style.textStyle);
  texted.style.width = `${m.w * zoom + 8}px`;
  texted.style.height = `${m.h * zoom + 4}px`;
}

function startText(existing: Extract<M.Shape, { kind: 'text' }> | null, p: M.Pt) {
  if (existing) adoptStyle(existing);
  editingText = { id: existing?.id ?? null, at: existing?.at ?? p };
  texted.value = existing?.text ?? '';
  texted.hidden = false;
  placeTextarea();
  texted.focus();
  render();
}

function commitText(cancel = false) {
  if (!editingText) return;
  const { id, at } = editingText;
  editingText = null;
  texted.hidden = true;
  const value = cancel && id === null ? '' : texted.value.replace(/\s+$/, '');
  const rest = hist.doc.shapes.filter((s) => s.id !== id);
  if (value) {
    const px = M.TEXT_PX[style.size] * u;
    const shape: M.Shape = { id: id ?? nextId++, kind: 'text', at, text: value, color: style.color, size: style.size, style: style.textStyle, ...measureText(ctx, value, px, style.textStyle) };
    const i = hist.doc.shapes.findIndex((s) => s.id === id); // keep z-order when re-editing
    const shapes = [...rest];
    shapes.splice(i >= 0 ? i : shapes.length, 0, shape);
    selectedId = shape.id;
    commit({ ...hist.doc, shapes });
  } else if (id !== null) commit({ ...hist.doc, shapes: rest });
  else render();
}

texted.addEventListener('input', autosize);
texted.addEventListener('keydown', (e) => {
  e.stopPropagation();
  if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    commitText();
  } else if (e.key === 'Escape') {
    e.preventDefault();
    commitText(true);
  }
});
texted.addEventListener('blur', () => commitText());

// ---------- crop ----------

function enterCrop() {
  tool = 'crop';
  crop = { draft: hist.doc.crop ?? full(), ratio: 'free' };
  selectedId = null;
  closePopover();
  $('#cropbar').hidden = false;
  fit();
  render();
}

function exitCrop(apply: boolean) {
  if (!crop) return;
  const r = { x: Math.round(crop.draft.x), y: Math.round(crop.draft.y), w: Math.round(crop.draft.w), h: Math.round(crop.draft.h) };
  crop = null;
  tool = 'select';
  $('#cropbar').hidden = true;
  if (apply && r.w >= 1 && r.h >= 1) {
    const whole = r.x === 0 && r.y === 0 && r.w === W && r.h === H;
    commit({ ...hist.doc, crop: whole ? null : r });
  }
  fit();
  render();
}

function startCropDrag(p: M.Pt) {
  if (!crop) return;
  const r = crop.draft;
  const corners: [M.Pt, M.Pt][] = [
    [{ x: r.x, y: r.y }, { x: r.x + r.w, y: r.y + r.h }],
    [{ x: r.x + r.w, y: r.y }, { x: r.x, y: r.y + r.h }],
    [{ x: r.x, y: r.y + r.h }, { x: r.x + r.w, y: r.y }],
    [{ x: r.x + r.w, y: r.y + r.h }, { x: r.x, y: r.y }],
  ];
  const corner = corners.find(([c]) => Math.hypot(c.x - p.x, c.y - p.y) <= 12 / zoom);
  const within = p.x > r.x && p.x < r.x + r.w && p.y > r.y && p.y < r.y + r.h;
  drag = corner ? { kind: 'crop-corner', fixed: corner[1] } : within ? { kind: 'crop-move', start: p, orig: r } : { kind: 'crop-new', start: p };
}

function moveCropDrag(p: M.Pt) {
  if (!crop || !drag) return;
  const ratio = crop.ratio === 'free' ? null : M.RATIOS[crop.ratio];
  if (drag.kind === 'crop-move') {
    const o = drag.orig;
    crop.draft = { ...o, x: Math.max(0, Math.min(W - o.w, o.x + p.x - drag.start.x)), y: Math.max(0, Math.min(H - o.h, o.y + p.y - drag.start.y)) };
  } else if (drag.kind === 'crop-new') crop.draft = M.cropFromDrag(drag.start, p, ratio, W, H);
  else if (drag.kind === 'crop-corner') crop.draft = M.cropFromDrag(drag.fixed, p, ratio, W, H);
  render();
}

$('#ratios').addEventListener('click', (e) => {
  const b = (e.target as Element).closest<HTMLElement>('[data-ratio]');
  if (!b || !crop) return;
  crop.ratio = b.dataset.ratio as M.Ratio;
  if (crop.ratio !== 'free') crop.draft = M.fitRatio(crop.draft, M.RATIOS[crop.ratio]);
  render();
});
$('#crop-apply').addEventListener('click', () => exitCrop(true));
$('#crop-cancel').addEventListener('click', () => exitCrop(false));

// ---------- save / copy / close / delete ----------

function ask(text: string, buttons: { label: string; value: string; primary?: boolean; danger?: boolean }[]): Promise<string> {
  return new Promise((resolve) => {
    $('#modal-text').textContent = text;
    const acts = $('#modal-acts');
    acts.replaceChildren(
      ...buttons.map((b) => {
        const el = document.createElement('button');
        el.className = b.primary ? 'b1' : 'b2';
        if (b.danger) el.classList.add('danger');
        el.textContent = b.label;
        el.onclick = () => {
          $('#modal').hidden = true;
          resolve(b.value);
        };
        return el;
      }),
    );
    $('#modal').hidden = false;
    (acts.lastElementChild as HTMLElement | null)?.focus();
  });
}

const fail = (what: string, e: unknown) => ask(`${what}: ${e}`, [{ label: 'OK', value: 'ok', primary: true }]);

async function save(): Promise<boolean> {
  try {
    await ipc.saveImage(await exportPng(img, hist.doc, u, W, H));
    hist.markSaved();
    render();
    return true;
  } catch (e) {
    await fail("Couldn't save", e);
    return false;
  }
}

/** Saves if needed, then puts path + file + image on the clipboard again. */
async function copy(): Promise<boolean> {
  if (hist.dirty) return save();
  try {
    await ipc.retryCopy(info.path);
    return true;
  } catch (e) {
    await fail("Couldn't copy", e);
    return false;
  }
}

async function done() {
  if (busy) return;
  busy = true;
  if (await copy()) await ipc.closeWindow();
  busy = false;
}

async function requestClose() {
  if (!hist.dirty) return ipc.closeWindow();
  const r = await ask(`Save changes to ${info.name}?`, [
    { label: 'Discard', value: 'discard', danger: true },
    { label: 'Cancel', value: 'cancel' },
    { label: 'Save', value: 'save', primary: true },
  ]);
  if (r === 'discard') await ipc.closeWindow();
  else if (r === 'save' && (await save())) await ipc.closeWindow();
}

async function remove() {
  const r = await ask(`Delete ${info.name}? This can't be undone.`, [
    { label: 'Cancel', value: 'cancel' },
    { label: 'Delete', value: 'delete', primary: true, danger: true },
  ]);
  if (r === 'delete') await ipc.editorDelete().catch((e) => fail("Couldn't delete", e));
}

void getCurrentWindow().onCloseRequested(async (e) => {
  e.preventDefault();
  await requestClose();
});

// ---------- chrome ----------

function undo() {
  if (hist.undo()) selectedId = null;
  render();
}

function redo() {
  if (hist.redo()) selectedId = null;
  render();
}

$('#undo').addEventListener('click', undo);
$('#redo').addEventListener('click', redo);
$('#delete').addEventListener('click', () => void remove());
$('#copy').addEventListener('click', () => void copy());
$('#done').addEventListener('click', () => void done());
$('#close').addEventListener('click', () => void requestClose());
$('#copy-path').addEventListener('click', () => void ipc.copyPath(info.path));
$('#reveal').addEventListener('click', () => void ipc.revealCapture(info.path));
$('#zoom-in').addEventListener('click', () => setZoom(M.zoomStep(zoom, 1)));
$('#zoom-out').addEventListener('click', () => setZoom(M.zoomStep(zoom, -1)));
$('#fit').addEventListener('click', () => {
  fit();
  render();
});
$('#rail').addEventListener('click', (e) => {
  const b = (e.target as Element).closest<HTMLElement>('button');
  if (!b) return;
  if (b.dataset.tool) setTool(b.dataset.tool as Tool);
  else if (b.id === 'swatch') {
    const s = selected();
    const t = s ? (s.kind as Tool) : tool;
    openPopover(SECTIONS[t].length ? t : 'arrow', b);
  }
});

const KEYS: Record<string, Tool> = { v: 'select', c: 'crop', a: 'arrow', r: 'rect', o: 'ellipse', l: 'line', p: 'pen', h: 'highlight', t: 'text', n: 'counter', b: 'redact' };

addEventListener('keydown', (e) => {
  if (editingText || !$('#modal').hidden) return;
  const mod = e.ctrlKey || e.metaKey;
  const k = e.key.toLowerCase();
  if (mod && k === 'z') {
    e.preventDefault();
    if (e.shiftKey) redo();
    else undo();
  } else if (mod && k === 'y') {
    e.preventDefault();
    redo();
  } else if (mod && k === 'c') {
    e.preventDefault();
    void copy();
  } else if (mod && k === 's') {
    e.preventDefault();
    void done();
  } else if (mod && (k === '=' || k === '+')) {
    e.preventDefault();
    setZoom(M.zoomStep(zoom, 1));
  } else if (mod && k === '-') {
    e.preventDefault();
    setZoom(M.zoomStep(zoom, -1));
  } else if (mod && k === '0') {
    e.preventDefault();
    fit();
    render();
  } else if (mod && k === '1') {
    e.preventDefault();
    setZoom(1);
  } else if (e.key === 'Enter') {
    e.preventDefault();
    if (crop) exitCrop(true);
    else void done();
  } else if (e.key === 'Escape') {
    if (popoverOpen()) closePopover();
    else if (crop) exitCrop(false);
    else if (selectedId !== null) {
      selectedId = null;
      render();
    } else void requestClose();
  } else if ((e.key === 'Delete' || e.key === 'Backspace') && selectedId !== null) {
    commit({ ...hist.doc, shapes: hist.doc.shapes.filter((s) => s.id !== selectedId) });
    selectedId = null;
    render();
  } else if (e.key.startsWith('Arrow') && selectedId !== null) {
    e.preventDefault();
    const d = e.shiftKey ? 10 : 1;
    const step: Record<string, [number, number]> = { ArrowLeft: [-d, 0], ArrowRight: [d, 0], ArrowUp: [0, -d], ArrowDown: [0, d] };
    const s = selected();
    const [dx, dy] = step[e.key] ?? [0, 0];
    if (s) replaceShape(M.translate(s, dx, dy));
  } else if (!mod && !e.altKey && KEYS[k]) setTool(KEYS[k]);
});

fit();
render();
```

- [ ] **Step 2: Type-check, test, build**

Run: `npm run build && npm test`
Expected: no `tsc` errors, and every test passes.

- [ ] **Step 3: Verify in the app against mockup 03**

With `npm run tauri dev` running, capture an area of text and bars (e.g. a browser page), then click the thumbnail. Check each item, comparing with `docs/design/mockups/03-editor.png`:

- **Opening:** the rail is on the left with Arrow active. The header shows the name, `W × H · PNG`, a disabled undo/redo, delete, Copy, Done and ×. The footer shows the path chip and `Fit`/zoom.
- **Drawing tools:**
  - Draw one of each: arrow, rectangle, ellipse, line, pen stroke, highlighter stroke (yellow, 40 %).
  - Shift gives 45° arrows and lines, and squares and circles.
  - Each new shape is selected with handles, and the handles resize it.
- **Style popover:**
  - Click the active Arrow button again. The popover shows colour, width, ends and shadow.
  - Changing the colour recolours the selected arrow, and changing the ends to "both" adds a second head.
- **Text:**
  - Choose Text, click, type two lines (Shift+Enter) and press Enter: a filled pill appears.
  - Double-click it with Select to edit it.
  - Change its style to Outline, then Plain.
- **Step counter:** three clicks make ①②③. Delete ② and click again: it makes ④.
- **Redact:**
  - Drag over some text for each of Pixelate, Blur and Solid.
  - After **Done**, open the saved PNG and check the text there is unreadable (zoom in with `eog`, or use python PIL to compare the region with the original).
- **Crop:** press `C`, then drag a smaller area, try 16:9, and **Apply ⏎**. The canvas shows the crop, the meta shows the new size, and undo restores the full image.
- **Select, move and edit:** with Select (`V`), click a shape to select it, drag to move it, use arrow keys to nudge, `Delete` to remove, and `Ctrl+Z`/`Ctrl+Shift+Z` to undo and redo.
- **Zoom:** `Ctrl+=`, `Ctrl+-`, the footer buttons, `Fit` and `Ctrl`+wheel all work, and drawing still lands under the pointer at 200 %.
- **Unsaved changes:**
  - The **Unsaved changes** marker appears after the first edit.
  - `Esc` with nothing selected → "Save changes?" → **Cancel** keeps the editor open.
  - Closing with **Discard** closes without writing: the file's mtime is unchanged.
- **Done:** it writes the file (mtime changes, cropped size), puts the new image on the clipboard (`xclip -selection clipboard -t image/png -o | file -` shows the new size) and closes.
- **Copy:** Copy with unsaved edits saves, stays open and clears the marker.
- **Delete:** delete → confirm → the file is gone and the window closes.
- **Footer:** the copy-path button puts only the text path on the clipboard (`xclip -selection clipboard -t TARGETS -o` lists no `image/png`), and the folder button opens Files.

Fix anything that fails before committing.

- [ ] **Step 4: Commit**

```bash
git add src/editor/main.ts && git commit -m "feat(editor): tools, selection, text, crop, zoom and save/close flows

Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>"
```

---

## Self-review notes

- **Spec §2.5 coverage:**

  | Spec item | Covered by |
  |---|---|
  | Header | Task 1 HTML, Task 4 wiring |
  | Rail and keys | Task 1 HTML, Task 4 `KEYS` |
  | Popover sections per tool | Task 3 `SECTIONS` |
  | Vectors until export, move/resize, `Del`, `Shift`, undo/redo | Tasks 2 + 4 |
  | Counter auto-numbering | `nextCounter` |
  | Crop presets and readout | Tasks 2, 3, 4 |
  | Redact changes the pixels | `render.redact` resamples the original, exports flattened |
  | Footer | Task 1 HTML, Task 4 wiring |
  | Done / `Ctrl+S` / `⏎` atomic save + re-copy + close | Tasks 1 + 4 |
  | Unsaved prompt | `requestClose` |
  | `Esc` order | keydown handler |
  | Thumbnail click and edit, tray "Open Last Capture" | Task 1 |

- **Deliberate simplification:** pen, highlighter, text and counter shapes can move but not resize (their handles list is empty). Resizing text means changing its size in the popover.
