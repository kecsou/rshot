# rshot — design spec

Date: 2026-09-22 · Status: approved design, pending spec review

rshot is a cross-platform (Linux, Windows, macOS) screenshot and screen-recording tool
written in Rust. It replaces the OS screenshot tool, including its shortcuts, and matches the
macOS tool's features. What sets it apart: **every capture puts its absolute file path on the
clipboard**, alongside the image and the file itself.

Approved mockups are the visual contract: `docs/design/mockups/0{1..5}-*.html` (+ `.png`
renders). Style **A · Glass** was chosen; B and C in `01-visual-direction` are rejected.

## 1. Goals and non-goals

**Goals (v1)**
- Capture area / window / full screen; record area / full screen to MP4 with optional mic.
- After every capture: file saved, clipboard set, floating thumbnail shown.
- Image editor (annotate, redact, crop) and video trimmer; both save **in place**.
- Take over the OS screenshot shortcuts (with consent) and give them back cleanly.
- Installers for Ubuntu/Debian, Fedora, other Linux, Windows, macOS, built in CI.
- First-class target: **Ubuntu 24.04, GNOME 46, X11** (the author's machine).

**Non-goals (v1)**: GNOME/KDE on Wayland; KDE/XFCE shortcut takeover (manual binding
documented instead); light theme; auto-update; GIF export; system-audio capture; pause while
recording; rotate, loupe tool, signature, OCR, pin-to-screen; re-editable annotations after
save; custom file-name patterns; selections spanning two monitors; code signing/notarization
(needs the owner's certificates; documented as follow-up).

## 2. Behaviour

### 2.1 Capture modes and default shortcuts

| Action | Linux (GNOME) | Windows | macOS |
|---|---|---|---|
| Overlay with toolbar (area mode) | `Print` | `PrtScn`, `Win+Shift+S` | `⌘⇧4`, `⌘⇧5` |
| Full screen, immediate | `Shift+Print` | `Win+PrtScn` | `⌘⇧3` |
| Active window, immediate | `Alt+Print` | `Alt+PrtScn` | — (`Space` in overlay) |
| Overlay in record mode / stop recording | `Ctrl+Alt+Shift+R` (taken over from Plan 3) | `Win+Shift+R` | — (toolbar) |

On macOS the Ctrl variants (`⌃⌘⇧3`, `⌃⌘⇧4`) map to the same actions. Shortcuts are
rebindable in Settings. "Full screen" means the monitor under the cursor. "Active window"
means the focused window.

### 2.2 Overlay (mockup 02)
- On trigger, rshot grabs every monitor into memory (**frozen frame**) and shows a fullscreen
  overlay per monitor. The toolbar appears on the monitor under the cursor.
- Toolbar: close · screen · window · area · record screen · record area · Options · Capture.
- Area mode: light dim, a crosshair, and a magnifier (pixel grid + coordinates + hex colour).
  Drag to select; 8 handles resize; drag inside to move; arrow keys nudge 1 px (Shift: 10 px);
  Shift while dragging = square; size label under the selection. `⏎` captures, `Esc` cancels,
  `Space` switches to window mode.
- Window mode: the window under the cursor is highlighted (blue tint + border + title/size
  label). Click captures that window.
- Options popover: save folder, timer (off/3/5/10 s), show thumbnail, remember last selection,
  show mouse pointer, microphone. Choices persist to config.
- Timer: the overlay hides. A countdown ring sits at the centre of the selection on the
  **live** screen, so you can open a menu or hover state during the countdown. When it ends,
  rshot grabs a fresh image of the same region, window or screen. `Esc` or a click cancels.
  (This departs from mockup 2d, where the ring sat over the dimmed frozen screen.)
- Hint bar (drag / Space / ⏎ / Esc) shows for the first 5 overlay openings, then never.

### 2.3 After a capture
1. Crop the frozen frame (area/screen) or capture the window (window mode; if that fails,
   fall back to cropping the frozen frame to the window rectangle).
2. If "show mouse pointer" is on, composite the cursor image.
3. Encode PNG, write it atomically (temp file + rename) to
   `<XDG Pictures>/Screenshots/Screenshot_YYYY-MM-DD_HH-MM-SS.png` (`_2`, `_3` … on a
   same-second collision). Videos: `<XDG Videos>/Screencasts/Recording_YYYY-MM-DD_HH-MM-SS.mp4`.
   Folders are created if missing and are configurable.
4. Set the clipboard (§2.4).
5. Play the shutter sound (setting) and show the floating thumbnail (setting).

**Floating thumbnail** (mockup 01-A bottom, 04c): glass card in the bottom-right of the active
monitor showing the image (or the video with its duration), "✓ Path copied" and the path.
It stays 5 s, and hover pauses the timer. Hover actions: edit, show in folder, delete. Click opens
the editor. Dragging it drops the file into other apps. Swiping it right dismisses it.

### 2.4 Clipboard contract
- **Path + image mode (default)**, one clipboard write with:
  - text: absolute path, e.g. `/home/kecsou/Pictures/Screenshots/Screenshot_2026-09-22_20-41-07.png`
  - files: `text/uri-list` + `x-special/gnome-copied-files` (Linux), `CF_HDROP` (Windows),
    file URL (macOS)
  - image: `image/png` bytes (images only; never for videos)
- **Path only mode**: the text entry alone.
- Linux X11 loses the clipboard when its owner exits. The rshot daemon therefore owns the
  clipboard until something else takes it.
- The editor's **Copy** and `Ctrl+C` write the same payload.

### 2.5 Image editor (mockup 03)
- Header: file name, size and format · undo · redo · delete · Copy · **Done** · close.
- Left glass rail. Tools with their keys: select V, crop C, arrow A, rectangle R, ellipse O,
  line L, pen P, highlighter H, text T, step counter N, redact B, plus the current colour swatch.
  Clicking the active tool again opens its style popover:
  - Colour: 8 swatches (`#ff453a #ff9f0a #ffd60a #30d158 #0a84ff #bf5af2 #fff #1c1c1e`) on
    every tool.
  - Width: 3 steps on arrow, line, shapes and pen.
  - Ends (one, both, none) and drop shadow: arrow.
  - Size S–XL and style (filled, plain, outline): text.
  - Mode (pixelate, blur, solid) and strength: redact.
- Objects stay vectors until export. Select to move or resize. `Del` deletes. `Shift`
  constrains angles and aspect. Undo/redo covers the whole session.
- Step counter numbers increase automatically: 1, 2, 3 …
- Crop: aspect presets (Free, 16:9, 4:3, 1:1), live size readout, Apply `⏎` or Cancel.
- Redact changes the exported pixels themselves, so the original content cannot be recovered.
- Footer: path chip (copy path, open folder) · unsaved marker · zoom − / % / + / Fit.
- **Done** (or `Ctrl+S`, `⏎`) flattens the image, writes the same path atomically, re-copies
  the clipboard and closes. Closing with unsaved changes asks "Save changes?". `Esc` first
  deselects, then closes.

### 2.6 Recording (mockup 04)
- Select a region on the frozen frame (or pick full screen). The overlay hides, a 3 s
  countdown runs (Esc cancels), then ffmpeg records the **live** screen.
- A dashed red frame is drawn just **outside** the region. It is a click-through, always-on-top
  window. The glass control pill sits outside the region and shows ● timer, a mic level meter,
  discard and **Stop**.
- Full-screen recordings: after the countdown the pill collapses to the tray (so it isn't
  filmed). The tray icon becomes a red "■ m:ss" and a click stops the recording.
- Stop by: the pill, the tray, or the record shortcut again. On stop: finalise the file, then
  the §2.3 steps 3–5 (the image entry is left out).
- ffmpeg records into `…/.Recording_…mkv` (MKV survives a crash) and remuxes to `.mp4`
  (`-c copy`) on stop, then deletes the MKV. If ffmpeg dies, the MKV is kept and the user is
  notified.
- Settings: microphone (none or a device), 30 or 60 fps. H.264 `yuv420p`, CRF 23, preset
  `veryfast`; AAC for the mic.

### 2.7 Video editor (mockup 04d)
- Player, filmstrip, yellow trim handles, playhead. Readout:
  `0:12.9 / 0:34.0 · keeping 0:04.8 → 0:28.6 (23.8 s)`. Mute-audio toggle.
- **Done** re-encodes the kept range (frame-accurate) to a temp file and renames it over the
  original, so the path stays the same.

### 2.8 Tray, first run, settings (mockup 05)
- Tray menu (native): Capture Area/Screen/Window, Record Screen…, Open Last Capture, Open
  Screenshots Folder, Settings…, Quit.
- First run: dialog "Make rshot your screenshot tool?" listing the shortcuts it will take
  over. **Use rshot** takes them over; **Not now** leaves the OS bindings alone (the tray still
  works). macOS adds a second step for the Screen Recording permission.
- Settings (one scrolling page): launch at login; screenshots and recordings folders;
  clipboard mode; floating thumbnail; shutter sound; take over system shortcuts (off = give
  them back); rebind each shortcut (inline "Press new shortcut…"); microphone; frame rate;
  version; "Open config file".

## 3. Architecture

### 3.1 Stack
- **Tauri 2** (Rust core, OS webview UI). The frontend is **plain TypeScript + CSS** (no
  framework) built with Vite as one multi-page app. It shares one `glass.css` for tokens and
  one `icons.svg` sprite; the mockup CSS is the starting point.
- Crates:
  - Tauri: `tauri` (tray-icon feature), `tauri-plugin-single-instance`,
    `tauri-plugin-autostart`, `tauri-plugin-dialog`, `tauri-plugin-notification`,
    `tauri-plugin-drag`.
  - Capture and clipboard: `xcap` (capture), `clipboard-rs` (multi-format clipboard),
    `image` (crop and PNG).
  - Config: `serde` + `toml`, `dirs`.
  - Per OS: `windows` (low-level keyboard hook) and `objc2*` (macOS extras) only where needed.
- ffmpeg is an **external program**:
  - Every platform, Linux included, ships a pinned static build as a Tauri sidecar
    (`externalBin`). Installers must be self-sufficient: they may depend only on libraries every
    stock desktop of that OS already has.
  - If no ffmpeg is found, the Record buttons are disabled with a tooltip explaining why.

### 3.2 Process model
- One resident process: `rshot` with no arguments, started at login. It holds the tray,
  the hidden, preloaded overlay windows (one per monitor), clipboard ownership, and the hotkey
  hooks (Windows/macOS).
- CLI: `rshot capture area|screen|window`, `rshot record`, `rshot restore-shortcuts`.
  A CLI call made while the daemon is running is forwarded to it through the single-instance
  plugin, and the CLI process exits at once. If no daemon is running, the CLI call starts one
  and then does the action. `restore-shortcuts` also runs without a daemon (used by
  uninstallers).
- GNOME custom keybindings invoke the CLI. Windows and macOS hotkeys fire inside the daemon.

### 3.3 Rust modules (`src-tauri/src/`)
| Module | Responsibility | Depends on |
|---|---|---|
| `main.rs` | arg parsing (std, no clap), Tauri builder, plugin wiring | all |
| `capture.rs` | monitors → frozen frames (`RgbaImage` + geometry + scale); crop; window list (geometry, z, focus) + window capture; cursor compositing | xcap, image |
| `clipboard.rs` | build and write the §2.4 payload (PNG passed as `Other("image/png")` so it isn't re-encoded) | clipboard-rs |
| `store.rs` | config load/save (TOML, defaults), capture folders, file naming, atomic write | serde, toml, dirs |
| `shortcuts/mod.rs` + `gnome.rs` / `windows.rs` / `macos.rs` | take over / restore / rebind; backup of the originals kept in config | per-OS |
| `recorder.rs` | build ffmpeg args per OS, spawn, stop (`q` on stdin), remux, list mics, trim | ffmpeg process |
| `ui.rs` | create, show and hide windows (overlay per monitor, thumbnail, editors, pill, settings, onboarding); position on the active monitor | tauri |
| `pipeline.rs` | finish a capture: PNG, atomic write, clipboard, sound, thumbnail; immediate captures; notifications | capture, store, clipboard, ui |
| `overlay.rs` | overlay session, frame IPC, capture targets, timer countdown | capture, pipeline, ui |
| `thumbnail.rs` / `settings.rs` | commands for the thumbnail, settings and onboarding windows | store, shortcuts, clipboard |

Pure logic (naming, crop math, gsettings list edits, ffmpeg args, clipboard payload, config
defaults) lives in plain functions with no I/O, so it can be unit-tested.

### 3.4 Frontend (`src/`)
`overlay/`, `thumbnail/`, `editor/` (canvas + object model), `video/`, `pill/`, `settings/`,
`countdown/`, `onboarding/`, and `shared/` (`glass.css`, `icons.ts`, `ipc.ts`). Frozen
frames reach the overlay as raw RGBA bytes through a binary IPC response
(`tauri::ipc::Response`). They never touch disk, and the canvas stays same-origin, so the
magnifier can read pixel colours.

### 3.5 Main flows
- **Screenshot:** trigger → `capture::grab_all()` → emit `overlay:show` → the overlay loads its
  frame → user selects → `capture_region(monitor, rect_css)` → Rust maps CSS px to physical px
  → crop → PNG → atomic write → clipboard → `thumbnail:show {path}` → overlays hide.
- **Immediate** (`Shift+Print`, `Alt+Print`): the same steps without the overlay.
- **Recording:** the overlay returns the region → hide → countdown → `recorder::start` → the
  frame window and pill appear → stop → finalise → clipboard → thumbnail.
- **Edit:** thumbnail click → editor loads the file → Done → canvas export (PNG bytes) →
  `save_image(path, bytes)` (atomic) → clipboard.

### 3.6 Shortcut takeover per OS
- **GNOME:**
  - Save the current values of `org.gnome.shell.keybindings` `show-screenshot-ui`,
    `screenshot`, `screenshot-window` and `show-screen-recording-ui` to config, then set them
    to `[]`.
  - Add custom keybindings under
    `org.gnome.settings-daemon.plugins.media-keys custom-keybindings`, at paths
    `/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/rshot-{area,screen,window,record}/`,
    that run
    `rshot capture …` / `rshot record`.
  - Restore puts the saved values back and removes only rshot's entries.
  - On the first spike, check that the overlay gets keyboard focus when launched this way
    (GNOME focus-stealing prevention). Forward `DESKTOP_STARTUP_ID` through single-instance and
    present the window with it.
- **Windows:**
  - Set `HKCU\Control Panel\Keyboard\PrintScreenKeyForSnippingEnabled=0`. Save the old value.
  - Install a `WH_KEYBOARD_LL` hook in the daemon that swallows and handles the four combos.
    `RegisterHotKey` can't claim `Win+Shift+S`, which the shell owns.
- **macOS:**
  - Disable symbolic hotkeys 28, 29, 30, 31 and 184 (`com.apple.symbolichotkeys`), then run
    `activateSettings -u`.
  - Register `⌘⇧3/4/5` as global hotkeys. Restore re-enables the saved entries.
- **Other Linux desktops:** Settings shows the manual commands to bind.

### 3.7 Packaging and distribution
| Target | Artefacts | Notes |
|---|---|---|
| Linux | `.deb` (Depends only on libraries present on stock Ubuntu desktops: libwebkit2gtk-4.1-0, libayatana-appindicator3-1, libpipewire, libgbm1; ffmpeg is bundled), `.rpm`, AppImage | built on ubuntu-22.04 for glibc reach; `prerm` restores shortcuts best-effort for logged-in users (per-user `gsettings` through their session bus) |
| Windows | NSIS `.exe`, per-user install | ffmpeg sidecar; the uninstall hook runs `rshot restore-shortcuts`; WebView2 bootstrapper |
| macOS | `.dmg`, universal binary | ffmpeg sidecar (per-arch static builds lipo'd); unsigned until the owner provides certificates; README documents the restore command, since drag-to-Trash runs no hook |

CI (GitHub Actions):
- `ci.yml` on every push: fmt, clippy, `cargo test`, frontend tests, and a build on
  ubuntu, windows and macos.
- `release.yml` on `v*` tags: `tauri-action` builds every artefact into a draft GitHub
  Release. ffmpeg sidecars are downloaded at build time from pinned URLs checked against a
  sha256.

Licensing: rshot MIT. ffmpeg (GPL build) ships as a separate executable with its licence and
source link in `THIRD_PARTY.md`.

## 4. Error handling
| Failure | Behaviour |
|---|---|
| Screen grab fails | system notification with the error; nothing written |
| Write fails (permissions, full disk) | put the **image** on the clipboard anyway, notify "Couldn't save to X: reason, image copied" |
| Clipboard write fails | file is kept; the thumbnail shows "Copy failed" with a retry action |
| ffmpeg missing | record buttons disabled, with a tooltip explaining how to install ffmpeg |
| ffmpeg exits mid-recording | keep the `.mkv`, notify with its path |
| Takeover fails (no gsettings, unknown desktop, macOS defaults error) | Settings toggle shows the error and the manual binding commands; the tray keeps working |
| Editor or trim save fails | original file untouched (atomic rename), error shown in the editor, changes kept open |
| Second launch | forwarded to the daemon (single instance) |

## 5. Performance targets (measured on the author's machine)
- Key press → overlay visible: **≤ 250 ms** (1080p and 4K).
- Mouse release → file saved + clipboard set: **≤ 300 ms** for a 4K region.
- Idle daemon (overlays preloaded): **≤ 200 MB RSS**. If it's more, create overlays on demand
  instead and re-measure against the 250 ms target.

## 6. Testing
- Rust unit tests (`cargo test`) for the pure functions:
  - naming and collisions;
  - CSS → physical crop with scale factors, offsets and clamping;
  - gsettings list edits, backup and restore;
  - ffmpeg args per OS (plus trim and remux);
  - clipboard payload (uri-list, gnome-copied-files);
  - config defaults and round-trip.
- Frontend: `vitest` for the pure modules only. These are the overlay selection geometry,
  shortcut-combo parsing, and the editor object model (hit testing, transforms, undo/redo,
  counter numbering, export order).
- Manual checklist on Ubuntu X11:
  - every shortcut, 2 monitors, HiDPI (scale 2);
  - pasting into gnome-terminal (path), Chrome/Slack (image) and Nautilus (file);
  - recording with and without the mic, a crashed recording, trim;
  - takeover, restore and uninstall restore.
- Windows/macOS: compiled and packaged in CI. **Not run by the implementer**; the owner smoke
  tests with the same checklist.

## 7. Delivery order
0. **Spike** (throwaway): preloaded Tauri overlay on GNOME X11 launched from a custom
   keybinding. Measure latency, focus and memory against §5. Decide preloaded vs on-demand.
1. **Screenshots on Linux:**
   - daemon, tray and CLI forwarding;
   - capture, overlay (area/window/screen, options, timer), clipboard, thumbnail;
   - GNOME takeover and onboarding, settings;
   - CI build on all three OSes from the start.
2. **Image editor.**
3. **Recording** + video editor.
4. **Windows and macOS:** takeover modules, ffmpeg args, sidecars, installers,
   `release.yml`. Push at the end.
