# rshot

Screenshot tool for Linux, Windows and macOS that replaces your OS screenshot tool,
shortcuts included. Every capture puts its **absolute file path** on the clipboard, next to
the file and the image, so a terminal or an AI agent gets a path and a chat app gets the picture.

## Install

Download the installer for your system from the GitHub release:

- **Ubuntu/Debian:** `sudo apt install ./rshot_<v>_amd64.deb`.
- **Fedora:** `sudo dnf install ./rshot-<v>-1.x86_64.rpm`.
- **Other Linux:** `chmod +x rshot_<v>_amd64.AppImage && ./rshot_<v>_amd64.AppImage`.
- **Windows:** run `rshot_<v>_x64-setup.exe` (per-user, no admin). SmartScreen may warn because the build is
  unsigned: **More info → Run anyway**.
- **macOS:** open `rshot_<v>_universal.dmg` (one app for Apple silicon and Intel), drag rshot to Applications,
  then run `xattr -dr com.apple.quarantine /Applications/rshot.app` (the app is unsigned). On first launch,
  allow Screen Recording when asked, then quit and reopen rshot. The grants (Screen Recording, and Microphone if you
  record it) belong to the exact build, so after each update grant them again: in System Settings → Privacy &
  Security, turn rshot off and on, then reopen it.

Start **rshot** once and choose **Use rshot**. It takes over these shortcuts:

| Action | Linux | Windows | macOS |
|---|---|---|---|
| Overlay: select an area, window (`Space`) or screen; `⏎` captures, `Esc` cancels | `Print` | `Win+Shift+S`, `PrtScn` | `⌘⇧4` (`⌃⌘⇧4`), `⌘⇧5` |
| Capture the monitor under the pointer | `Shift+Print` | `Win+PrtScn` | `⌘⇧3` (`⌃⌘⇧3`) |
| Capture the focused window | `Alt+Print` | `Alt+PrtScn` | not set |
| Record the screen | `Ctrl+Alt+Shift+R` | `Win+Shift+R` | not set |

`PrtScn` on Windows and `⌘⇧5` on macOS always open the overlay; the others can be changed in Settings.

Captures are saved as `Screenshot_YYYY-MM-DD_HH-MM-SS.png` in the `Screenshots` folder inside your Pictures folder.
Settings live in the tray menu and in `config.toml`: `~/.config/rshot/` on Linux, `%APPDATA%\rshot\` on Windows,
`~/Library/Application Support/rshot/` on macOS.

## Record the screen

The record shortcut (or the record buttons in the overlay's toolbar) → pick an area or the whole screen → 3-second
countdown → recording. Stop with the pill's **Stop**, the tray's **Stop Recording**, or the same shortcut again.
Recordings are saved as `Recording_YYYY-MM-DD_HH-MM-SS.mp4` (H.264, optional microphone) in `Screencasts` inside your
Videos folder (Movies on macOS), their path is copied, and clicking the thumbnail opens a trimmer. rshot ships its
own ffmpeg (`rshot-ffmpeg`; see `THIRD_PARTY.md`, installed with rshot), so nothing else needs installing.

## Give the shortcuts back

Turn off **Settings → Take over system screenshot shortcuts**, or:

- **Linux:** run `rshot restore-shortcuts`. While rshot is running, the running app does the restore; if it fails,
  it says so in a notification. Uninstalling the `.deb` or `.rpm` does this for every logged-in user. The AppImage
  has no uninstaller: before deleting it, turn the takeover off or run `./rshot_<v>_amd64.AppImage restore-shortcuts`.
- **Windows:** uninstalling rshot does it (the uninstaller closes rshot first). By hand: quit rshot from the tray,
  then run `"%LOCALAPPDATA%\rshot\rshot.exe" restore-shortcuts`.
- **macOS:** dragging rshot to the Trash runs nothing, so before that: quit rshot from the menu bar, run
  `/Applications/rshot.app/Contents/MacOS/rshot restore-shortcuts`, and delete
  `~/Library/LaunchAgents/rshot.plist` (there if **Launch at login** was on).

If the `.deb` or `.rpm` was removed while you were logged out, give GNOME its keys back and remove rshot's four
custom shortcuts (or delete the four **rshot** entries in Settings → Keyboard → View and Customize Shortcuts →
Custom Shortcuts):

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

## Other Linux desktops

Automatic takeover works on GNOME. Elsewhere, bind `rshot capture area|screen|window` and
`rshot record` in your desktop's keyboard settings.

## Known limitations

- The pointer can be included in screenshots on Linux only (recordings always show it).
- macOS and Windows builds are unsigned (the macOS app has an ad-hoc signature only).
- GNOME/KDE on Wayland aren't supported yet.
- On macOS, uninstalling by dragging to the Trash doesn't restore ⌘⇧3/4/5, so first quit rshot and run
  `/Applications/rshot.app/Contents/MacOS/rshot restore-shortcuts` (see above).
- macOS: giving the keys back re-enables ⌘⇧3/4/5 with their default keys, so a remap you had made of those
  shortcuts in System Settings is lost.
- macOS: a letter shortcut (for example `⌘⌥A`) binds the key that types that letter in your keyboard layout, even
  though ⌥ makes it type another character. After switching layouts, restart rshot to rebind letters.
- Windows: a `Ctrl+Alt+<letter>` shortcut also swallows `AltGr+<letter>` (Windows sends AltGr as Ctrl+Alt), so on
  layouts like AZERTY it can block characters such as `€`.
- Windows: Settings can't record `Win+…` or `PrtScn` combos, because the shell takes them before rshot sees them.
  Write them into `[shortcuts]` in `%APPDATA%\rshot\config.toml` instead (for example `record = "Super+Shift+R"`),
  then restart rshot.
- Windows: the trimmer plays the video from rshot over `http://127.0.0.1`. If a WebView2 update starts blocking
  local-network requests, it shows "Can't play this video here"; the file and its copied path are unaffected.
- Fedora: the trimmer needs an H.264 decoder. The `.rpm` recommends `gstreamer1-plugin-openh264` (Fedora's Cisco
  openh264 repository) as a best effort; without it, recordings still save and copy.
- Release builds come from `.github/workflows/release.yml`: push a `v*` tag (draft GitHub release), or run it
  manually from the Actions tab (installers as workflow artifacts).

## Owner smoke test (Windows and macOS)

The Windows and macOS builds are compiled and packaged by CI but were never run by the implementer. First, before the
first `v*` tag, run `release.yml` once from the Actions tab (`workflow_dispatch`): it exercises the NSIS hook, `lipo`
and the `.dmg`, the Git Bash tools (`unzip`, `shasum`) and the AppImage on Ubuntu 22.04, and returns the installers as
workflow artifacts. Then run the manual checklist in the design spec (§6) on each OS, plus:

- **Windows:**
  - Installer: per-user install without admin, the WebView2 bootstrapper on a machine without WebView2; uninstall
    gives PrtScn and Win+Shift+S back; installing a newer version over an older one keeps the takeover on.
  - Whether Windows applies the `PrintScreenKeyForSnippingEnabled` change live, without signing out.
  - The Settings shortcut recorder with AltGr (Ctrl+Alt) on an AZERTY layout in WebView2.
  - gdigrab region recordings on mixed-DPI multi-monitor setups: offsets and size (BtbN's `ffmpeg.exe` DPI
    awareness; `ddagrab` is the fallback).
  - The microphone list: BtbN n8.1's dshow `"name" (audio)` lines.
  - The trimmer plays the loopback video stream (WebView2 Local Network Access).
  - Left-clicking the tray icon stops a recording.
  - Quit gives PrtScn and Win+Shift+S back to Windows; starting rshot again takes them again.
  - Holding PrtScn down captures once, not once per key repeat.
  - Killing `rshot.exe` (Task Manager) mid-recording stops its ffmpeg too (Job object).
- **macOS:**
  - The universal `.dmg` on Apple silicon and Intel; `xattr`; Screen Recording and Microphone prompts, and after an
    update.
  - Whether `activateSettings -u` applies the symbolic hotkeys without logging out on macOS 14+.
  - Screen Recording already denied: the first **Open System Settings** click may do nothing, the second opens the
    pane.
  - The menu bar may flash before it auto-hides as each overlay appears; simple fullscreen `screen()` on a freshly
    shown overlay; the Dock and menu bar come back after a multi-monitor capture; overlays over a full-screen app's
    Space.
  - Mixed-scale multi-monitor placement of the overlays, countdown, frame, pill and thumbnail.
  - Rebinding a shortcut in Settings (the global shortcuts pause meanwhile).
  - avfoundation recordings: "Capture screen N" order and frame size against rshot's screen list and image sizes
    (Retina), and whether `AVCaptureScreenInput` still works on current macOS.
  - Whether the firewall prompts when rshot's loopback video server listens; the trimmer plays the video (ATS).
  - Left-clicking the menu bar icon stops a recording.
  - Quit gives ⌘⇧3/4/5 back to macOS; starting rshot again takes them again.
  - The microphone, saved by name, is still found after unplugging and replugging it.

## Develop

Prerequisites: Rust stable, Node 22, and on Ubuntu:

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential libxdo-dev libssl-dev libayatana-appindicator3-dev \
  librsvg2-dev libpipewire-0.3-dev libspa-0.2-dev clang libclang-dev libgbm-dev libdrm-dev libwayland-dev \
  libxcb1-dev libxcb-randr0-dev libxcb-shm0-dev libxcb-xfixes0-dev \
  patchelf gstreamer1.0-plugins-base gstreamer1.0-plugins-good gstreamer1.0-libav  # for the AppImage
npm ci
scripts/fetch-ffmpeg.sh  # the ffmpeg sidecar: after cloning and whenever the pinned build changes (idempotent)
npm run tauri dev        # run
npm test                 # frontend unit tests
(cd src-tauri && cargo test)
npm run tauri build      # .deb, .rpm and AppImage in src-tauri/target/release/bundle/
```

On Windows, run `scripts/fetch-ffmpeg.sh` from Git Bash; `npm run tauri build` makes the NSIS installer. On macOS,
`TARGET=universal-apple-darwin scripts/fetch-ffmpeg.sh` then `npm run tauri build -- --target universal-apple-darwin`
(with both Rust targets installed) makes the universal `.dmg`. If the AppImage step complains about FUSE, rerun it with
`APPIMAGE_EXTRACT_AND_RUN=1`. `scripts/cross-check.sh` type-checks the Windows and macOS code from Linux. After moving
`scripts/fetch-ffmpeg.sh` to newer ffmpeg builds, follow the steps at the top of that script.

Design: `docs/superpowers/specs/2026-09-22-rshot-design.md`; mockups in `docs/design/mockups/`.
