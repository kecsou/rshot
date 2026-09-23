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

## Give the shortcuts back

Turn off **Settings → Take over system screenshot shortcuts**, or run:

```bash
rshot restore-shortcuts
```

Uninstalling the `.deb` does this for every logged-in user. If it was removed while you were
logged out:

```bash
gsettings reset org.gnome.shell.keybindings show-screenshot-ui
gsettings reset org.gnome.shell.keybindings screenshot
gsettings reset org.gnome.shell.keybindings screenshot-window
```

## Other desktops

Automatic takeover works on GNOME. Elsewhere, bind `rshot capture area|screen|window` in your
desktop's keyboard settings.

## Develop

Prerequisites: Rust stable, Node 22, and on Ubuntu:

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential libxdo-dev libssl-dev libayatana-appindicator3-dev \
  librsvg2-dev libpipewire-0.3-dev libspa-0.2-dev clang libclang-dev libgbm-dev libdrm-dev libwayland-dev \
  libxcb1-dev libxcb-randr0-dev libxcb-shm0-dev libxcb-xfixes0-dev
npm ci
npm run tauri dev        # run
npm test                 # frontend unit tests
(cd src-tauri && cargo test)
npm run tauri build      # .deb in src-tauri/target/release/bundle/deb
```

Design: `docs/superpowers/specs/2026-09-22-rshot-design.md`; mockups in `docs/design/mockups/`.
