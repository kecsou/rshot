# rshot

Screenshot tool for Linux (Windows and macOS next) that replaces your OS screenshot tool,
shortcuts included. Every capture puts its **absolute file path** on the clipboard, next to
the file and the image, so a terminal or an AI agent gets a path and a chat app gets the picture.

## Install (Ubuntu / Debian)

```bash
sudo apt install ./rshot_0.1.0_amd64.deb
```

Start **rshot** once from the app grid and choose **Use rshot**.

| Shortcut | Action |
|---|---|
| `Print` | Overlay: select an area, window (`Space`) or screen; `⏎` captures, `Esc` cancels |
| `Shift+Print` | Capture the monitor under the pointer |
| `Alt+Print` | Capture the focused window |

Captures are saved as `~/Pictures/Screenshots/Screenshot_YYYY-MM-DD_HH-MM-SS.png`.
Settings live in the tray menu and in `~/.config/rshot/config.toml`.

## Record the screen

`Ctrl+Alt+Shift+R` (or the record buttons in the `Print` toolbar) → pick an area or the whole screen → 3-second
countdown → recording. Stop with the pill's **Stop**, the tray's **Stop Recording**, or the same shortcut again.
Recordings are saved as `~/Videos/Screencasts/Recording_YYYY-MM-DD_HH-MM-SS.mp4` (H.264, optional microphone),
their path is copied, and clicking the thumbnail opens a trimmer. rshot ships its own ffmpeg (`rshot-ffmpeg`; see
`THIRD_PARTY.md`), so nothing else needs installing.

## Give the shortcuts back

Turn off **Settings → Take over system screenshot shortcuts**, or run:

```bash
rshot restore-shortcuts
```

While rshot is running, the running app does the restore; if it fails, it says so in a notification.
Uninstalling the `.deb` does this for every logged-in user. If it was removed while you were
logged out, give GNOME its keys back and remove rshot's four custom shortcuts (or delete the
four **rshot** entries in Settings → Keyboard → View and Customize Shortcuts → Custom Shortcuts):

```bash
gsettings reset org.gnome.shell.keybindings show-screenshot-ui
gsettings reset org.gnome.shell.keybindings screenshot
gsettings reset org.gnome.shell.keybindings screenshot-window
gsettings reset org.gnome.shell.keybindings show-screen-recording-ui
k=org.gnome.settings-daemon.plugins.media-keys
gsettings set $k custom-keybindings "$(gsettings get $k custom-keybindings |
  sed -E "s#'[^']*/rshot-(area|screen|window|record)/'##g; s#(, )+#, #g; s#\[, #[#; s#, \]#]#")"
for id in area screen window record; do
  dconf reset -f /org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/rshot-$id/
done
```

## Other desktops

Automatic takeover works on GNOME. Elsewhere, bind `rshot capture area|screen|window` and
`rshot record` in your desktop's keyboard settings.

## Develop

Prerequisites: Rust stable, Node 22, and on Ubuntu:

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential libxdo-dev libssl-dev libayatana-appindicator3-dev \
  librsvg2-dev libpipewire-0.3-dev libspa-0.2-dev clang libclang-dev libgbm-dev libdrm-dev libwayland-dev \
  libxcb1-dev libxcb-randr0-dev libxcb-shm0-dev libxcb-xfixes0-dev
npm ci
scripts/fetch-ffmpeg.sh  # once: downloads the bundled ffmpeg sidecar
npm run tauri dev        # run
npm test                 # frontend unit tests
(cd src-tauri && cargo test)
npm run tauri build      # .deb in src-tauri/target/release/bundle/deb
```

Design: `docs/superpowers/specs/2026-09-22-rshot-design.md`; mockups in `docs/design/mockups/`.
