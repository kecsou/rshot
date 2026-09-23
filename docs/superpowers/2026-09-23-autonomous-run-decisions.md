# rshot autonomous run: decisions for review

Plans 1–4 (Linux screenshots, image editor, recording, Windows/macOS and installers) ran subagent-driven on 2026-09-22/23, directly on `main`.
Every task had an implementer and a reviewer, and each plan ended with a final whole-plan review and one fix wave. The controller made every ruling below on its own while you were away. You can overturn any of them.
Sources: `.superpowers/sdd/2026-09-22-plan-{1..4}-*/` (progress.md, deferred.md, preflight.md, final-fixes.md, task rulings and reports).
Status (updated 2026-09-23 after 15:04): all four plans are complete at HEAD `a24d7d2`. The Plan 4 final fix wave landed as `b2d7d5e`, `39d8ffe`, `5b31ad3` and `4649aa5`, and its residuals as `a24d7d2`; every item that an earlier draft marked pending is resolved below.

"Cost if wrong" means what you lose if the ruling turns out to be the wrong call, or what it takes to reverse it.

## 1. Rulings that change what you asked for or approved

These rulings depart from the spec, the mockups or the plans in a way you would notice.

| # | Ruling (plan/task) | Departs from | Cost if wrong |
|---|---|---|---|
| 1 | **Spec §5 perf targets recalibrated** (P1 T6, spec edited in `9b2a5f2`). The old targets were key → overlay ≤ 250 ms (hard) and an idle daemon ≤ 200 MB RSS, with on-demand overlays as the fallback. The new ones are ≤ 250 ms mean with occasional runs to ~260 ms, and ≤ 320 MB PSS after use. Measured: mean ≈ 229 ms, 34/37 runs ≤ 250 ms, max 257 ms; T13 later saw a median ≈ 262 ms (machine load). On-demand overlays measured ~500 ms and lost focus, so they were rejected. Accepted on top of that: +150 ms (307–357 ms) when a thumbnail card is on screen (P1 final), and +150 ms for a screenshot taken during an area recording (P3 T3). | Spec §5 | Closing the last ~5 % in latency and memory needs a native (non-WebView) overlay, which is out of scope. |
| 2 | **Video preview through a loopback HTTP server instead of Tauri's asset protocol** (P3 T4). WebKitGTK 2.52 can't play `<video>` from `asset://` (its GStreamer source takes only http/https/blob), and the blob fallback loads the whole file into memory. The editor now streams from a std-only range server: 127.0.0.1 on a random port, a random 128-bit token per editor mapped to the canonical path, `Host` must equal `127.0.0.1:<port>`, GET/HEAD with a single range, at most 32 connections, a time budget for headers, and the token is dropped when the editor closes. CSP `media-src http://127.0.0.1:*`; the asset protocol was removed. The thumbnail card shows a **poster PNG plus the duration** (made by the bundled ffmpeg), not the playing video that spec §2.3 describes. | Plan 3; spec §2.3 (video in the card) | The daemon runs a local listener. Any local process can hold all 32 slots, which blocks playback only (accepted residual). The token comes from std `RandomState`, not a dedicated CSPRNG call (accepted with a `ponytail:` note). macOS needs an ATS exception and WebView2 may start blocking it (see §7). |
| 3 | **Mic level meter built** (P3 D10). The plan dropped it, saying it was "recorded as a ruling", but no ruling existed. Spec §2.6 and mockup 04 show it. ffmpeg gets an `astats`/`ametadata` filter printing RMS dB to stdout, a reader thread parses it, and the pill draws the meter. | Plan 3 constraints ("not built") | One extra ffmpeg filter and a stdout reader thread. To drop it, remove the filter and the meter. |
| 4 | **One universal macOS dmg** (P4 E7). The plan built per-arch dmgs; the spec says universal. One macos-14 job builds `universal-apple-darwin` with a `lipo`'d ffmpeg sidecar. | Plan 4 | CI churn to go back. Every Mac user downloads both architectures. |
| 5 | **The shortcut recorder refuses bare F-keys and Shift+letter on every OS** (P4 T2). One `combo::takeable` rule (Ctrl, Alt or Super required, unless the key is Print) now applies to the Windows hook, the macOS bindings and the Settings recorder on Linux too. Since `39d8ffe` (Plan 4 final fix #13), `to_gnome_accel` also refuses such combos in hand-edited Linux configs; keysyms like `Ctrl+Home` still work. | Plan 1 behaviour on Linux | Nobody can bind a bare F-key (or Shift+letter) any more, on any OS. |
| 6 | **GNOME restore resets keys to their defaults.** P1 final I#3: a backed-up `@as []` is restored with `gsettings reset`, and so is a taken key that has no backup and is currently `[]`. P3 T5: restore now resets each key first and only sets a value when the backup differs from the default, so restored keys keep no dconf user value. | Plan 1 (restore wrote the backup back verbatim) | A `[]` or a default-equal value you had set on purpose comes back as "GNOME default" rather than as an explicit setting. Functionally the same for default values. |
| 7 | **The Windows "key already off" rule is now "trust the observed value", with a macOS copy of the backup** (P4 final review, landed in `39d8ffe` and `a24d7d2`). **This reverses the earlier Plan 4 Task 3 lost-config ruling** ("missing backup + key off ⇒ restore to enabled"). Windows: if Snipping-on-PrtScn was already off (`0`) before the takeover, restore leaves it off (the ShareX/Greenshot case). macOS: a disabled hotkey is backed up as `0`; when the hotkeys can't be read, existing backups are kept, so a saved `0` never becomes `1`; the read is scoped with `plutil -extract AppleSymbolicHotKeys`. A second copy of the backup goes into rshot's own defaults domain (`io.github.kecsou.rshot hotkeyBackup`) before the first change. Release, restore and the CLI `restore-shortcuts` use that copy when config.toml has no backup, and a restore deletes it. | Plan 4 T3 ruling (reversed) | On Windows, a lost config.toml leaves classic PrtScn (copy the screen) instead of Snipping. On macOS, after a lost config the copy is deleted only by a restore, so every later Quit applies it again and reverts any hand edit of ⌘⇧3/4/5 made meanwhile. |
| 8 | **Quit on Windows/macOS gives the keys back but keeps your consent, and the NSIS uninstall hook does the same** (P4 E28/E23: `restore-shortcuts --keep-consent`). Relaunching (or autostart) takes the keys again, and upgrades no longer switch the takeover off. The plan's uninstaller ran the plain `restore-shortcuts`. The Plan 4 final wave extended this. Since `b2d7d5e`, ⌘Q, macOS logout and Windows sign-out/shutdown (`RunEvent::Exit`) also give the keys back and stop a running recording: ffmpeg gets 2 s, and the MKV is recovered at the next launch. Since `4649aa5`, an NSIS pre-install hook does the same keep-consent release for "Don't uninstall" and `/UPDATE` upgrades. | Plan 4 | An upgrade keeps the takeover on. That's intended, but surprising if you expected a reinstall to reset it. |
| 9 | **`hardenedRuntime: false` and ad-hoc signing (`signingIdentity "-"`) on macOS** (P4 T5, E35). The implementer found that the hardened runtime needs the `com.apple.security.device.audio-input` entitlement for mic capture (rshot and the sidecar), and without notarization it adds nothing. The ad-hoc seal isn't Developer ID signing, so the build still counts as "unsigned". | Tauri default; spec "unsigned" | When you get a Developer ID, turn the hardened runtime back on with an entitlements plist for rshot and the sidecar. Screen Recording/Microphone grants reset after every update (the README says so). |
| 10 | **Licensing items deferred to you** (P4 T5). Releases attach FFmpeg, x264, both build-script trees and a generated `FFMPEG-NOTICES.txt`. Left for you: corresponding source (or a written offer) for the other GPL libraries in the builds (x265, xavs2, davs2, xvid, rubberband, vid.stab, frei0r, zvbi, fftw3, libdvdread/nav…), the lame/xvid licence files, the licences that live only in AviSynth+/nv-codec-headers/libglvnd sources, and the macOS library versions. The list is now under "Before the first public release" in the README owner section (moved out of the shipped `THIRD_PARTY.md` in `4649aa5`). | Spec §3.7 licensing | A GPL compliance gap on the first public release if it isn't done. |
| 11 | **Windows and macOS were never run** (P4 setup). Verification there is type-checking (`scripts/cross-check.sh`), unit tests and CI on push. The builds are unsigned. | Expectation that everything is tested | Platform bugs only show up in your smoke test (§7). |
| 12 | **"Path only" clipboard mode serves only the three text targets** (P1, plan Task 3 amended in `31ce3ef` to match spec §2.4 "the text entry alone"). | Plan 1 text | Path-only users can't paste the file into Files. |
| 13 | **Captures over 15 MiB go to the clipboard without `image/png` on X11** (P1 T3/T4). clipboard-rs has no INCR, so a bigger property silently breaks clipboard serving for good. Path and file entries are still served. | Spec §2.4 payload | A very large capture pastes as a path/file only, with no image in chat apps. |
| 14 | **Pasting in Files makes "Pasted image.png"** (P1 T13). Nautilus 46 prefers `image/png` over the file list. Accepted, because dropping `image/png` would break image paste into Chrome/Slack. | Spec expectation that Files pastes the file | A Files paste gives a duplicate PNG under another name. |
| 15 | **Opaque popups** `rgba(40,40,46,.92)` instead of the mockups' blurred glass (P1 T9), for every popup over the live desktop, because WebKitGTK can't blur the desktop behind a window. | Mockups 01/04/05 | Less translucency than the mockups. |
| 16 | **Coarser redaction than the brief** (P2 T3): block = `round((12 + 20·strength)·u)` px in both modes, and the rect snaps outward to whole pixels. The brief's floor left text readable in WebKitGTK. Parked alongside: pixelate flattens thin text, and WebKit samples pixels instead of averaging them, so blocks look harsh. | Plan 2 brief values | Redaction looks coarser at low strength. |
| 17 | **Pill Discard takes two clicks** (P3 final #2): the first click arms it (red, 3 s), the second discards. | Mockup 04 (one click) | One extra click. |
| 18 | **An interrupted recording that can't be remuxed becomes a visible `Recording_*.mkv`** and is notified once (P3 final #1, beyond the brief). Recoverable ones are remuxed at the next start and notified. | Spec §4 (keep the hidden MKV and notify) | To keep it hidden, delete 4 lines; it would then be re-notified at every start. |
| 19 | **Frame and pill are override-redirect on Linux** (P3 T3). They skip GNOME's fade/animation in recordings and screenshots, and they're hidden while an overlay is up and around every grab. The pill can't take keyboard focus: stop with the tray or the shortcut. | Plan 3 | No keyboard access to the pill. |
| 20 | **Record buttons disabled with a tooltip when ffmpeg is missing, and the tray shows a red `■ m:ss`** (P3 D9/D11). The spec won over the plan's "notify an error" and `● m:ss`. | Plan 3 | None. |
| 21 | **`.deb` Recommends `gstreamer1.0-libav`** (P3 D21). **The `.rpm` recommends `gstreamer1-plugin-openh264` (unverified, best effort), and the AppImage bundles GStreamer** (P4 E27). All three are there so the trimmer can play H.264. | Spec's dependency list | Without the codec, the trimmer shows "Can't play this video here"; the file and clipboard still work. The AppImage is bigger. |
| 22 | **Post-review residual fixes were checked by the controller only.** `b51f0e2` (Plan 2: AltGr guard, native copy in the text box, Quit raises editors) and `c065624` (Plan 3: editor button focus, devtools grant in dev, narrower recovery, "still saving" notice) were each the base of the next final review, so no reviewed diff contained them. **Resolved:** the controller added both to the Plan 4 fix-wave re-review, which found them OK. The same pattern repeats once more: `a24d7d2` (Plan 4 residuals: macOS CLI restore without config, one release per Quit, poison-safe lock at exit, 8 smoke-list lines) was read by the controller only, and no review follows it. | "Every task reviewed" | One small diff, `a24d7d2`, has no second reader. Its code changes are Windows/macOS-only branches, type-checked but with no unit test. |
| 23 | **Linux is built on ubuntu-24.04, so the `.deb`, `.rpm` and AppImage need glibc 2.39** (after the first CI run, `35865046332`). On ubuntu-22.04, `libspa` 0.10.1 (xcap → `pipewire`) doesn't compile against PipeWire 0.3.48 headers (no `spa_meta_region_is_valid`/`spa_meta_first`, no `spa_video_info_raw.flags`, `modifier` i64 instead of u64). CI and the release job both moved to ubuntu-24.04; the README and spec §3.7 say glibc 2.39 (Ubuntu 24.04+). | Plan 4 / spec §3.7 ("built on ubuntu-22.04 for glibc reach") | Ubuntu 22.04, Debian 12 and other pre-2024 distros can't run the Linux packages. Getting them back means pinning xcap/pipewire to versions that build on PipeWire 0.3.48, or building in an older container with newer PipeWire headers. |

## 2. Plan 1: foundation and Linux screenshots

| Ruling | Why | Cost if wrong |
|---|---|---|
| Work directly on `main` in the main checkout, no worktree (Plans 2–4 repeat this). | You authorised commits and a push at the end; a worktree complicates running the tray app locally. | History is on main instead of a branch, which is easy to branch or rebase later. |
| Every subagent inherits the session model (Plans 2–4 repeat this). | Your global CLAUDE.md overrides the skill's cheap-model choice. | Token spend only. |
| The controller commits only between implementer runs. | A plan fix (`31ce3ef`) was committed while Task 1 ran, which shifted the review base. | None. |
| "Path only" = the three text targets only (plan amended). | Spec §2.4. | Path-only users can't paste the file into Files. |
| An invalid config.toml is moved aside to `config.toml.invalid`; only NotFound silently defaults (T2). | The plan's code overwrote it and lost `gnome_backup`; spec §3.6/§4 require restorable shortcuts. | One extra file on disk. |
| Skip `image/png` above 15 MiB on X11 (T3 → T4). | clipboard-rs has no INCR; a bigger PNG silently kills clipboard serving. | Huge captures paste as a path/file only. |
| xcap returning no monitors (`swap_remove` panic) upgraded to Important and fixed (T4). | A panic in dispatch kills the tray daemon. | None (one line). |
| `[profile.dev.package."*"] opt-level = 2` (T5). | Debug captures took ~3 s, so interactive checks weren't representative. | A slower first dev build. |
| rshot sets `WEBKIT_DISABLE_DMABUF_RENDERER=1` on Linux when it's unset (T6). | This machine's NVIDIA driver/library mismatch makes WebKitGTK abort with GBM EGL errors; the variable is the common Tauri workaround. | Slightly more CPU for webview compositing, for every Linux user. |
| Task 5 deviations accepted: `set_focusable` so only the overlay under the pointer takes focus; async `overlay_cancel`. | Enter captured the wrong monitor in ~3/10 sessions; after Esc the next overlay lost its first key. | None. The async cancel was verified only with synthetic zero-duration keys. |
| The T6 gate failed (279–353 ms, 743 MB summed RSS), so: optimise rather than relax. Forward argv over the single-instance D-Bus before Tauri/GTK init, grab monitors in parallel, have non-active overlays wait for `overlay:primary-ready`, tune malloc (`mallopt(M_MMAP_THRESHOLD)` replaced `malloc_trim`), and measure memory as PSS. | Summed RSS counts shared WebKit libraries three times. | ~100 MB more idle RAM than planned. |
| Accept the committed build and recalibrate spec §5 (see §1 row 1). | The remaining gap needs a native overlay. | Overlay ~5 % slower and ~5 % bigger than the original targets. |
| T7 fixes, all plan-mandated code: mode shared across monitors, overlay no longer stuck with Esc disabled after a rejected IPC, stale non-active load hidden. | Spec §2.2 multi-monitor behaviour; never trap the user. | A small extra diff. |
| T8: 5 departures from the brief accepted. The grid uses `minmax(0,1fr)` so the Save-to path ellipsizes; the toggles stack in the left column (mockup 2c); the loupe hides while options are open; `popup` dropped `.resizable(false)` so the countdown ring is centred; the GTK folder dialog is made transient via rfd internals (50 ms poll, 3 s ceiling). | Each was found in interactive checks. | The dialog adoption depends on rfd internals and fails silently after 3 s. |
| T9: the path guard accepts only the last capture (Plan 2 extends it to open editors); a stale card closes when thumbnails are disabled; `allowed()` is pure and tested. | The old guard covered a whole folder the webview can reconfigure: arbitrary read, delete, open. | None (security). |
| T9: opaque card background `rgba(40,40,46,.92)` for every popup over the live desktop. | The footer was illegible; WebKitGTK can't blur the desktop. | Less glass than the mockup. |
| T9 minors included: the card no longer stays always-on-top if `run()` fails; only the left button opens the viewer. | Trapping UI and a surprising action. | None. |
| T10: check gsettings stderr (it exits 0 on a failed dconf write); save the backup to disk before changing keys; per-key backup; reject unknown modifiers instead of mapping them to `<Super>`; one Exec-safe quoting helper. | The plan's verbatim code could clear your only backup. | A small extra diff. |
| T11 fix round: Settings reloads on focus (a stale snapshot overwrote overlay changes); onboarding 400 px with a scrollable key list (room for Plan 3's 4th row); rollback keeps `takeover` on a failed restore; no `innerHTML` in onboarding; "Not now" gets a catch. | Correctness and XSS with IPC access. | A small extra diff. |
| T12 accepted: `DOCS_RS=1` in cross-check; monitors grabbed in parallel on Linux only (`HMONITOR` isn't `Send`). | The macOS check can't run here otherwise. | Windows/macOS grab monitors one after another (slower with several monitors). |
| Nautilus "Pasted image.png" accepted (T13). | Keeping `image/png` matters more for Chrome/Slack. | A duplicate PNG when pasting in Files. |
| T13 fix round: the README's logged-out recovery also removes rshot's custom keybindings; `restore_from_cli` skips users with no config (prerm loops over gdm etc.); `bundle.publisher` set. | User-facing correctness. | None. |
| Final review: ONE fix wave. I#1 keep rshot's own UI out of the next capture (`clear_own_ui`); I#2 route `restore-shortcuts` through a running daemon; I#3 reset keys whose backup is `@as []`. Minors: `~`/relative folders, the " (deleted)" exe suffix after upgrades, the onboarding Close double handler, slow commands made async with a lock-order note, a race-free capture name (hard-link placement), `delete_capture` clearing stale state, countdown IPC wrappers, a relaunch from the app grid opening Settings. | Data safety, correctness, one pass. | A larger diff to re-review. |
| Final: plan-level items fixed in the plan texts before Plans 2/3 ran. Plan 3 must not widen the guard (asset scope per file); Plan 2's `open_editor`/`save_image` async; Plan 3 stale text, `command_for`, a Record arm with no panic. | Cheaper to fix before execution. | None. |
| Final: CSP and per-window command allow-lists deferred to a hardening task at the start of Plan 3. | They had to land before the asset protocol. | Defence in depth one plan later (done in P3 T0). |
| Final: measure 4K save+clipboard time if cheap. | M#10. | Measured on a 2557×1597 area only: 33–73 ms against 300 ms. 4K and scale 2 weren't measured. |
| Final fix-wave deviations accepted: the prescribed Windows test manifest in build.rs was NOT added (it breaks the build, and the bin already embeds the Common-Controls v6 manifest); `dismiss_thumbnail` was removed in favour of `close_window`. | Evidence in the fix report. | If the reasoning is wrong, Windows `cargo test` fails with STATUS_ENTRYPOINT_NOT_FOUND on first CI. |
| Final residual (a Windows test asserting `/data/shots` is absolute) carried into Plan 2 Task 1 instead of a second fix wave. | One fix wave per plan. | Windows CI red until then. Fixed in `979949b`. |
| Final parked: a Settings window open during a forwarded restore can re-take the shortcuts on its next unrelated change (it also shows the old toggle). | Rare: it needs a restore while Settings is open. | Shortcuts re-taken; toggle them off. |
| Final parked: key → visible is +150 ms while a thumbnail card is up. | The trade-off of keeping the card out of the next capture. | 307–357 ms instead of ~190 ms in that case. |

## 3. Plan 2: image editor

| Ruling | Why | Cost if wrong |
|---|---|---|
| Same setup as Plan 1: `main`, session model. | As Plan 1. | As Plan 1. |
| Editor windows get `ui::force_focus` after `show()` on Linux. | Mutter ignores `set_focus` (Plan 1 T6/T13). | None. |
| Every page imports `../shared/base` first (context menu suppressed in production). | Plan 1 T11 rule. | None. |
| T1 deviations accepted: canonical paths in the editors map with a direct compare; entries pruned on `WindowEvent::Destroyed`; `copy_path` async; `editor_delete` closes a stale thumbnail. | Found during implementation. | None. |
| The editors map stores `(opened, canonical)`: the opened path for clipboard/display, the canonical one for the guard and dedupe (T1 minors → T4). | Keep "the path never changes" when the folder is behind a symlink. | A small Rust change. |
| Carried to T4: seed pen/highlighter strokes with their first point (empty points → infinite bounds); clamp the crop drag start inside the image. | T2 review. | None. |
| Ellipse hit test uses a 64-point polyline distance, not the plan's formula (T2). | The formula over-reached on elongated ellipses; spec select/move behaviour wins. | None (more accurate). |
| Fractional crop rects are rounded on commit (`exitCrop`); no action. | Already in the T4 brief. | None. |
| Redaction block `round((12 + 20·strength)·u)` for both modes via pure `model.redactBlock`, with an outward snap (T3). | Spec §2.5 "cannot be recovered"; a probe showed the brief's floor left text readable. | Coarser redaction at low strength. |
| `styles.ts` querying `#pop` at import is fine; the outline `measureText` overhang, `pad()` dedupe, rect shadow and box-filter downscale are parked as cosmetic (T3). | Only main.ts imports styles.ts. | Small visual imperfections. |
| Carried to T4: pass `u` into `drawDoc`/`drawShape`; wait for the Inter font before the first `measureText`. | T3 review. | None. |
| The delete prompt focuses the first non-danger button, and `markSaved` only runs if the doc didn't change during the export (T4). | Enter deleted the file; a save race marked an unwritten edit as saved. Data loss outranks the verbatim brief. | None. |
| T4 minors all taken: narrower mousedown `preventDefault`, fit reserves room for the crop bar, `:focus:not(:focus-visible)`, tiny new-crop restore, `isComposing`, symlink refusal test. | Cheap; UI/UX is a stated priority. | None. |
| Parked: pixelate flattens thin text. | Cosmetic; the spec only requires unrecoverable. | Harsh-looking blocks. |
| Final review: ONE fix wave with every finding plus a PNG-signature check. Critical: redaction did nothing when its rect overshot the image. Important: a clipboard failure after a good write was shown as "Couldn't save". Plus 16 minors. | Data loss, redaction and UX are all spec-bound. | A larger diff to re-review. |
| Implementer concerns accepted as is: save/delete act on the canonical target (Delete through a symlink to a single file removes the target); close is ignored while a prompt is up; the keyboard-layout fallback applies only when `e.key` isn't Latin. Also noted: Ctrl+C inside the text box runs Copy unless text is selected. | The re-review agreed. | Edge-case behaviour you might not expect. |
| Re-review residuals (Windows AltGr letters triggering Done/Copy, native copy with a selection, raise editors on Quit) went to the implementer as a follow-up (`b51f0e2`), and the controller read the diff instead of a new review round. | Plan 2 had no rounds left. | Resolved: the Plan 4 fix-wave re-review later reviewed `b51f0e2` and found it OK (§1 row 22). |

## 4. Plan 3: recording

| Ruling | Why | Cost if wrong |
|---|---|---|
| Same setup as Plan 1: `main`, session model. | As Plan 1. | As Plan 1. |
| The pre-flight scan was delegated to a read-only subagent (preflight.md, D1–D34). | Keeps the controller's context lean. | A missed conflict surfaces in task review. |
| Every suggested fix in preflight.md was accepted unless noted (table below). | Spec binding. | See the D table. |
| D10: build the pill's mic level meter. | Spec §2.6 and mockup 04; the plan's "ruling" didn't exist. | One filter plus a reader thread. |
| D21: `Recommends: gstreamer1.0-libav`, and the video page never hangs (an error shows "Can't play this video here"; Copy/Reveal still work). | WebKitGTK needs it for H.264; apt installs Recommends by default. | Without the universe repo, no preview; file and clipboard still work. |
| D25: pin a dated BtbN autobuild (n8.1.2-50, autobuild-2026-08-31-13-27) with a sha256 check; add `pulse` to the capability checks; reuse the scratchpad build if the hash matches. | Spec §3.7; `latest` rebuilds daily. | A URL bump when the pin expires (~2 years). |
| D9/D11: the spec wins. Record buttons are disabled with a tooltip when ffmpeg is missing (tray item too); the tray shows `■ m:ss` with a red icon. | Spec §3.1/§4/§2.6. | None. |
| D17: `trim_args` moves to T4, `even` is Linux-gated, and a temporary `#[allow(dead_code)]` sits on `mod recorder;` for T1 only. | `clippy -D warnings` and cross-check would fail. | None. |
| Dispatches carry Plan 2's post-fix-wave signatures (`guard` → `(given, canonical)`, `save_image` → `Result<bool>`, `open_editor` closes thumbnails). | Plan 2's wave ran at the same time. | Compile errors, caught by the implementer. |
| T0 review carries: a cargo test that build.rs's command list equals `generate_handler!`; a note that stale `autogenerated/*.toml` files must be deleted by hand; `dialogs.json` split into `settings.json` and `onboarding.json`. | Least privilege and drift protection. | None. |
| Parked: `core:default` breadth (later fixed in the final wave, item 9). | Plan-mandated at the time. | Fixed. |
| New constraint: no `style=""` markup on pages that have a `<style>` element (the CSP nonce disables `'unsafe-inline'`). | T0 finding. | None. |
| T1 carries: `-cluster_time_limit 1000 -flush_packets 1` so a crash keeps most of the MKV; the PATH fallback for ffmpeg always runs, with an exec-bit check; `rsplit_once(']')` for mic names; fetch-script hardening (trap, `--retry 3`, version check). | T1 review. | None. |
| Carry-map correction: D23 and the gnome.rs array changes move from T2 to T5. | `TAKEN_KEYS` grows in T5. | None. |
| ffmpeg gets `PR_SET_PDEATHSIG` through a spawner thread whose lifetime equals ffmpeg's (T2 I1). | A killed rshot left ffmpeg recording invisibly. Privacy and disk. | fork instead of posix_spawn (+ms). |
| T2: all minors taken (tray race guard, MKV path on errors after the end, CLI stop off the zbus thread, atomic take-or-quiet, quitting flag, `read_capture` PNG-only). Accepted as is: killing ffmpeg with SIGKILL loses ~2 s (a consequence of the spec'd encoding); a missing mic falls back to the default mic; name reservation and the tray-disable extras. | Cheap. | The accepted items are real, small losses. |
| T2 fix round accepted: a hard-killed rshot loses the MKV tail, and a run under ~1.5 s may leave an empty MKV; the tray resets only when the slot is empty. | Same class as the accepted crash loss. | Lost last second(s) after a hard kill. |
| Carried to T3: the stdout drain reads to EOF (the drain thread exiting would SIGTERM ffmpeg); the prctl argument cast and return check; a recorder-level `stopping` flag that Quit waits on. | T2 re-review. | None. |
| T3: fix `monitor_at` at the root (physical containment over the available monitors; the thumbnail was affected too); hide and re-show the frame/pill around every grab. | At scale 2 multi-monitor the wrong monitor was picked; the frame and pill landed in screenshots. | +150 ms on screenshots taken during an area recording. |
| T3: minors 1 (work area), 2 (a frame error never blocks the pill), 3 (record modes disabled while recording), 4 (NO_FFMPEG text from Rust, per OS), 6 (tests) taken; 5 (200 ms post-countdown race) and 7 (X shape region) parked. | Cheap vs cosmetic. | See §6 residuals. |
| T3 deviations accepted: override-redirect frame (and later pill), ignore-cursor after show, remember the selection after clamping, pill clamped to the monitor, Record checks for ffmpeg first. | Avoid GNOME animations in recordings. | The pill has no keyboard focus. |
| Parked to the final wave: override-redirect pill/frame stacking above an open overlay (Discard clickable over the frozen frame); overlapping grabs re-showing early; `unavailable()` ignoring ENDING. | Found in the T3 review. | Fixed in the final wave (items 3, 11, 12). |
| Parked, accepted: the pill has no keyboard focus (tray and shortcut stop it). | Override-redirect. | Keyboard-only users stop via the shortcut. |
| T4, revised before review: poster PNG card and a loopback range server instead of the asset protocol (details in §1 row 2). | WebKitGTK can't play `asset://` video; blobs don't scale. | A local token-gated listener, ~150 lines. |
| T4 deviations accepted: `trim_video` returns `Result<bool>`; `save_image` accepts PNG only and `trim_video` MP4 only (both editors share the `editor-*` capability). | Defence in depth. | None. |
| T4: loopback-server limits (about 32 connections, Builder spawn, accept-error backoff, header time budget, a `respond()` unit test) plus minors 1–8 (abort on empty output, a non-blocking poster with a timeout, drag without a poster, stream failure → fallback, anchored `parse_duration`, raw-bytes poster IPC, Ctrl+C = Copy). Minor 10 (a `RandomState` token) accepted with a ponytail note. | No resource limits: unbounded threads, slowloris, EMFILE spin. | See the residual below. |
| Accepted residual: a local process can hold the 32 stream slots (≤ 10 s each). | Loopback only. | Playback DoS only. |
| Commit trailer: every commit carries Co-Authored-By plus the Claude-Session line, passed in each dispatch (the classifier blocked editing the template). | System reminder. | None. |
| T5 minors taken anyway: restore = reset, then set only if different (plus a custom-keybindings reset when empty); unplugged mic shown as "(unavailable)"; overlay keys don't hijack a focused select; overlay options broadcast across monitors; shared select CSS. Accepted: no mic icon in the native select; the empty-backup → reset rule unchanged. | Leaves no dconf trace, and fixed this machine's explicit-default values in one round trip. | See §1 row 6. |
| T5 follow-up: `mousedown` `preventDefault` + blur on `#bar`/`#pop` buttons so clicks don't steal Space/arrows. | Found in T5. | None. |
| Parked to the final wave: toggle label clicks keeping focus; `closeOptions` blurring a focused control. | T5 re-review. | Fixed (final item 13). |
| T6 → final wave: atomic licence copy, THIRD_PARTY.md wording, README names `/usr/lib/rshot/THIRD_PARTY.md`, "# once" → rerun after pin bumps. | T6 review. | Fixed (items 14–15). |
| Carried to Plan 4: DEP-5 `/usr/share/doc/rshot/copyright`, a repo `LICENSE` (MIT), releases attaching ffmpeg/x264 source and notices, the Linux release built on ubuntu-22.04 (the local binary needs GLIBC_2.39); later moved to ubuntu-24.04 (§1 row 23). | GPL compliance and glibc reach. | Done in P4 T5 (partly deferred to you, §1 row 10). |
| Final review: ONE fix wave with all 10 findings plus 5 parked items. Important: interrupted recordings never surfaced; one-click Discard. | Data loss, privacy indicator, UX. | A larger diff to re-review. |
| Final wave accepted: an unrecoverable orphan becomes a visible `Recording_*.mkv`; overlapping grabs handled by design; a header double-click not maximizing under xdo is pre-existing drag.js behaviour. | Re-review agreed. | See §1 row 18. |
| Residuals ruled in and verified by the controller only (`c065624`): button mousedown in both editors, `core:webview:allow-internal-toggle-devtools` (dev builds only inject it), recovery narrowed to `.Recording_` temps and skipping MKVs modified < 5 s ago, and a notice when Record is pressed while a save runs. | Small. | Resolved: the Plan 4 fix-wave re-review later reviewed `c065624` and found it OK (§1 row 22). |
| Parked: label-wrapped switch rows in the editors ("Mute audio", "Drop shadow") may keep focus after a click on the label text. | Minor. | Space/Enter may toggle the switch instead of the editor shortcut. |

**Pre-flight items D1–D34** (all accepted as suggested unless a ruling above says otherwise):

| Item | Accepted fix (task) | Cost if wrong |
|---|---|---|
| D1 | The thumbnail allow-list grants `close_window` (`dismiss_thumbnail` isn't a command) (T0) | None; without it no card could close |
| D2 | The icon sprite uses `hidden`, not `style="display:none"`, under the CSP nonce (T0) | None |
| D3 | All 4 recording commands listed in build.rs; `list_mics` granted to the overlay and Settings (T2/T5) | None |
| D4 | `trim_video` allow-listed (T4) | None |
| D5 | `retry_copy` sends bytes as `image/png` only for PNGs, never for videos (T2) | None |
| D6 | `new_recording_path` with `_2`/`_3` suffixes, claimed race-safely (T2) | None |
| D7 | The recordings folder goes through `absolute()`; one generalised `dir_setting` (T2/T5) | None |
| D8 | `finish_video` shares the capture tail, shutter sound included (T2) | The shutter sound also plays when a recording stops (if enabled) |
| D9 | Ruled above | — |
| D10 | Ruled above | — |
| D11 | Ruled above | — |
| D12 | `clear_own_ui` also clears `pending_rec` (T3) | None |
| D13 | The `overlay_record` error path notifies and resets `busy` (T3) | None |
| D14 | Check, spawn and insert under one lock; watchdog first; a pill failure is notified, not fatal (T2) | None |
| D15 | `recording_discard` async (T2) | None |
| D16 | Quit while recording stops ffmpeg first (T2) | Quit waits for the stop |
| D17 | Ruled above | — |
| D18 | The frame margin scales with the monitor (T3) | None |
| D19 | Asset scope on the canonical path (T4) | Moot: the asset protocol was removed |
| D20 | `crossOrigin` for the video drag icon (T4) | Superseded by the poster card |
| D21 | Ruled above | — |
| D22 | The Settings snippet goes through `update()`; mic list loaded async (T5) | None |
| D23 | A user already taken over gets the record key taken at startup (T5) | Ctrl+Alt+Shift+R changes owner on the first launch of the new version, without a prompt |
| D24 | The README manual restore adds the 4th key and `record` (T6) | None |
| D25 | Ruled above | — |
| D26 | A shared `ask()` and CSS for both editors (T4) | None |
| D27 | Time rounding (`0:60.0` → `1:00.0`) (T4) | None |
| D28 | Trim tests seen red first; recordings-dir test (T4/T5) | None |
| D29 | Every overlay mode test covers `recarea`/`recscreen` (T3) | None |
| D30 | Plan-text corrections (T0–T5) | None |
| D31 | The watchdog mentions the MKV only if it exists; ffmpeg.log is appended to (T2) | ffmpeg.log grows over time |
| D32 | THIRD_PARTY.md and the FFmpeg licence shipped in the `.deb` (T1/T6) | None |
| D33 | Mic list refreshed on open; pill clamped to the monitor (T3/T5) | None |
| D34 | The sidecar must exist before any Linux cargo command (T1) | Build prerequisite: run `scripts/fetch-ffmpeg.sh` first |

## 5. Plan 4: Windows, macOS, installers

| Ruling | Why | Cost if wrong |
|---|---|---|
| Same setup as Plan 1: `main`, session model. | As Plan 1. | As Plan 1. |
| Pre-flight delegated to a read-only subagent (E1–E43), run while Plan 3 Task 6 was in review. | Context; read-only, so no conflict. | A missed conflict surfaces in review. |
| Windows/macOS runtime can't be tested here: verification = cross-check clippy, unit tests and CI on push; unsigned builds, documented. | Linux-only machine. | Platform bugs found by your smoke test. |
| Every suggested fix in preflight.md accepted (table below). | Spec binding. | See the E table. |
| E7: one universal dmg. | The spec says universal. | CI churn. |
| E35: ad-hoc signing (`"-"`). | Consistent with "unsigned"; it seals the bundle. | One config line. |
| E27: rpm recommends `gstreamer1-plugin-openh264`, unverified, marked best effort in the README. | Fedora's ffmpeg-free has no H.264 decoder. | An rpm install warning at most. |
| T1 review carries: a `default_shortcuts_parse` test (Linux literals pinned); a combo `combo::parse` rejects is reported as a takeover error, never skipped; no bare-key combo in the Windows hook; `manual_commands` drops empty shortcuts. | A hand-edited config could swallow a key system-wide. | None. |
| T2: one shared `combo::takeable` (Ctrl, Alt or Super, or Print) enforced in the Windows bindings, the macOS arm and the Settings recorder on all OSes. | Takeover and the recorder disagreed: Shift+letter swallowed capitals; a bare F-key recorded fine, then the takeover rolled back entirely. GNOME steals those keys too. | Nobody can bind a bare F-key. |
| T2: fix a pre-existing layout bug. The recorder takes letters/digits from `e.key` when Latin, else from `e.code` (AZERTY recorded the wrong letter). | Found in review. | None. |
| T2: minors 2 (time-based repeat), 3 (refresh the Snipping backup when the current value ≠ 0), 4 (`outdated()` re-arms on Windows), 5 (wording), 6 (unpause on Settings destroy) taken. Accepted: 7 (the hook is installed even when the takeover is declined), 10 (logoff skips the release). README notes for AltGr and the Windows "Bind these yourself" copy. | Cheap. | Minor 10 was fixed by final fix #1 (`b2d7d5e`): sign-out now releases the keys through `RunEvent::Exit`. |
| → final wave: letters only from `e.key` (`/^[a-z]$/i`), digits from `e.code`; `to_gnome_accel` enforces `takeable`; the Windows `backup(None, Some(0))` case. | T2 re-review. | Done in `5b31ad3` (recorder) and `39d8ffe` (GNOME accels; `backup(None, Some(0))` now gives `"0"`). |
| → T3: global-hotkey on macOS binds physical keycodes, so letter labels must map to the current layout (or be documented). | AZERTY Macs. | Done: letters bind by the current layout; restart rshot after switching layouts (README). |
| T3: fix all three Importants. Logical placement from xcap points, a cursor → points hit-test, simple fullscreen entered in `overlay_ready` after show and left in reverse order, and a macOS rebind pause by unregistering/registering on the main thread without waiting. | A stuck auto-hide menu bar on multi-monitor, mixed-scale placement with a possible nil-screen panic, no rebind pause. | None. |
| T3: lost config on both OSes: a missing backup with the key off ⇒ restore to enabled/absent. | "Safer for a screenshot tool". | **Reversed** by the final review ("trust the observed value" row below), landed in `39d8ffe`. |
| T3: minors taken (cfprefsd read, rollback serialisation, onboarding label, no double permission window, `LSUIElement`, SAFETY comment). Accepted: a registration failure leaves the toggle ON (retried at the next launch). README notes: `-dict-add` replaces remapped hotkeys; ⌥+letter binds by label. | Cheap. | Until the next launch the toggle says ON while the keys don't work. |
| → final wave: macOS `read_hotkeys` scoped with `plutil -extract AppleSymbolicHotKeys json`; keep existing backups when the domain can't be read. | Other `<data>`/`<date>` values in the domain could break parsing. | Done in `39d8ffe` (`defaults export … \| plutil -extract AppleSymbolicHotKeys json`). |
| → T4: frame and pill placement in points on macOS. | Mixed-scale Macs. | Done in T4. |
| T4: store the macOS mic by label and resolve it to the current avfoundation index at record time; error if it's gone. | Device order changes on plug/unplug (wrong mic, or failure). | None. |
| T4 minors taken: `-pixel_format uyvy422`; cache only a successful Job object; dshow multi-type lines; doc fixes; macOS countdown/thumbnail/editor placed in points; proportional avfoundation crop. The fix round simplified placement to px everywhere, with macOS dividing by the target monitor's scale at `set_position`/`set_size` (checked against the tao source). | T4 review. | Not run on a Mac. |
| T5: `.gitattributes` `eol=lf` for `*.sh`/`*.nsh`; one pre-remove script for deb and rpm (`remove`, `purge`, `0`); actions pinned by SHA; `includeUpdaterJson: false`; README AppImage restore and smoke-list additions; DEP-5 source path. Accepted: `hardenedRuntime: false`. Implementer additions accepted: a generated `FFMPEG-NOTICES.txt`, `.sha256` sidecar stamps, a `shasum` → `sha256sum` fallback. | CRLF would break `fetch-ffmpeg.sh` on windows-latest; a duplicate script. | See §1 row 9. |
| T5: deferred to you (README/THIRD_PARTY note): full corresponding-source mirroring, lame/xvid notices, macOS library-version notices. | Out of reach for the run. | §1 row 10. |
| → final wave: move "Before the first public release (owner)" from the shipped THIRD_PARTY.md to the README; SHA-pin `dtolnay/rust-toolchain` (with `toolchain: stable`) and `Swatinem/rust-cache`. | THIRD_PARTY.md ships to users. | Done in `4649aa5`. Every action in both workflows is SHA-pinned, with `persist-credentials: false`. |
| Final review: trust the observed value on the first takeover (Windows `0`, macOS `0`), plus a secondary macOS backup copy in rshot's own defaults domain. This reverses the T3 lost-config ruling. | Re-enabling what a user had disabled (common with ShareX/Greenshot) is worse. | Landed in `39d8ffe`. A lost config.toml on Windows leaves classic PrtScn instead of Snipping; for the macOS side effect see §1 row 7. |
| Final review: ONE fix wave with everything marked "→ final fix wave". I1: `RunEvent::Exit` (⌘Q, logout, WM_ENDSESSION) skips the key release and the recording stop on Windows/macOS. I2: the backup rule. I3: smoke-list gaps. Plus 8 minors. | One pass. | Landed as `b2d7d5e`, `39d8ffe`, `5b31ad3` and `4649aa5`. The re-review found all 13 items addressed. Exit behaviour, the backup rule and the NSIS hooks are type-checked and unit-tested only. |
| Add `b51f0e2` and `c065624` to the fix-wave re-review scope, and add the two dropped Windows notes to the smoke list. This came from gaps this document found. | Both commits were controller-only (§1 row 22); the notes had fallen out of Plan 3's carry list. | Both commits reviewed OK. The notes are in the README (trim over a streamed file, visible temp files). |
| Residuals after the re-review, verified by the controller only (`a24d7d2`). N2: on macOS, `restore-shortcuts` with no config.toml restores from the defaults copy and deletes it, without saving a config (Linux/Windows keep the early return that prerm relies on). N1: a `RELEASED` flag, set when Quit's release succeeds, so the exit doesn't release a second time. N3: a poison-safe lock when stopping a recording at exit. Plus 8 README smoke-list lines. | Small, and all at the OS boundary. | No subagent review and no unit test (§1 row 22). If N1 is wrong, an exit after a successful Quit skips a release that was already done; a failed Quit release still retries at exit. |
| First CI run (`35865046332`): windows-latest and macos-latest passed, ubuntu-22.04 failed to compile `libspa` 0.10.1 against PipeWire 0.3.48 headers. Build Linux on ubuntu-24.04 in ci.yml and release.yml; the release build step sets `APPIMAGE_EXTRACT_AND_RUN=1` so linuxdeploy doesn't need FUSE 2. | Local 24.04 builds work; 22.04's PipeWire is too old for the capture crate. | Linux packages need glibc 2.39 (§1 row 23). The AppImage step on 24.04 only runs in release.yml, so it's unproven until you run it (§7). |

**Pre-flight items E1–E43** (all accepted as suggested unless a ruling above says otherwise):

| Item | Accepted fix (task) | Cost if wrong |
|---|---|---|
| E1 | No `protocol-asset`/`image-png` features; tray template via `include_image!` (T3) | None; the plan's version broke the build |
| E2 | macOS stubs stay until T3 (T2) | None |
| E3 | `even()` on every OS (T4) | None |
| E4 | The macOS hotkey handler dispatches on a spawned thread (T3) | None; otherwise the UI freezes and the thumbnail ends up in grabs |
| E5 | One `ffmpeg_command` helper with `CREATE_NO_WINDOW` for every ffmpeg spawn (T4) | None; otherwise a console window appears in recordings |
| E6 | Fetch script per triple: pinned URL + sha256 + per-OS capability checks; BtbN win64; Martin Riedl 9.0.2 for macOS; GPLv3 text pinned for macOS (T5) | macOS ffmpeg comes from a third-party builder; pins need manual bumps |
| E7 | Ruled above | — |
| E8 | A shared `encode_tail` (crash-safe flags, mic level filter) for all OS builders (T4) | None |
| E9 | Windows/macOS packages ship THIRD_PARTY.md and FFMPEG-LICENSE.txt (T5) | None |
| E10 | cross-check sets `externalBin` to null via `TAURI_CONFIG` (T5) | None |
| E11 | `request_screen_permission` in build.rs and onboarding.json (T3) | None |
| E12 | Dead-code allow in combo.rs, narrowed to Linux in T3 (T1/T3) | None |
| E13 | Store tests compare against per-OS defaults (T1) | None |
| E14 | macOS takeover: `defaults` run synchronously; (un)registration posted to the main thread without waiting; a failure notifies and rolls back (T3) | None; otherwise a possible deadlock under the config lock |
| E15 | Takeover failures reported (Windows hook result, macOS registration) (T2/T3) | None |
| E16 | macOS: each hotkey's `enabled` bit backed up and saved before disabling; restore writes it back (T3) | Restore uses default parameters, so a remap is lost (README) |
| E17 | macOS frame origins converted consistently (later points-based placement) (T3/T4) | Untested on mixed-scale Macs |
| E18 | One `idle_icon`; template flag kept for idle, off for the red icon (T3) | None |
| E19 | Info.plist `NSAllowsLocalNetworking` for the loopback stream (T3) | If macOS 26.5+ blocks loopback anyway, the trimmer can't play; fallback `NSAllowsArbitraryLoads` |
| E20 | All `Region` literals fixed (T4) | None |
| E21 | `record_args(..., full, ...)`; no `return` in tail position (T4) | None |
| E22 | The avfoundation screen index is resolved before the recording lock, with a 5 s timeout (T4) | None |
| E23 | The NSIS pre-uninstall hook runs `restore-shortcuts --keep-consent` (T2/T5) | Upgrades keep the takeover on |
| E24 | NSIS `CheckIfAppIsRunning` before the hook; README says to quit rshot first on macOS (T5) | None |
| E25 | The Windows hook swallows auto-repeat and the matching key-up (T2) | None |
| E26 | Windows Job object kills ffmpeg with rshot; macOS has no equivalent (ponytail ceiling) (T4) | macOS: a killed rshot leaves ffmpeg recording until the disk fills |
| E27 | AppImage bundles GStreamer; rpm pre-remove restore; rpm recommends openh264 (T5) | A bigger AppImage; openh264 unverified |
| E28 | Quit on Windows/macOS releases the OS keys but keeps consent (T2/T3) | After Quit, the OS keys work again until rshot starts |
| E29 | `cargo fmt --check` in every verify step (all) | None |
| E30 | README note and smoke item for WebView2 Local Network Access (T5) | If LNA gets enabled, the trimmer shows "Can't play"; file and clipboard unaffected |
| E31 | Release workflow: a `create-release` job, patchelf and GStreamer installed, no `rpm` from apt (T5) | None |
| E32 | Files lists completed (T2/T3/T5) | None |
| E33 | `pointer_supported` in `OverlayInfo`; the toggle hidden with `hidden` (T3) | None |
| E34 | OS-neutral copy ("the system's shortcuts"); onboarding lists the hard-wired key; relaunch hint (T3) | None |
| E35 | Ruled above | — |
| E36 | Simple fullscreen left when overlays hide (T3) | None |
| E37 | F13–F24 parse correctly; the ⌃ variant only for ⌘⇧3/4 (T3) | None |
| E38 | Tray left-click stops a recording on Windows/macOS (menu otherwise) (T3) | While recording, left-click no longer opens the menu |
| E39 | README Windows/macOS restore sections; the macOS NO_FFMPEG hint says "Reinstall rshot" (T5) | None |
| E40 | Windows backup saved before the registry write; one `start` signature; no Mutex around the Sender (T2) | None |
| E41 | Owner smoke list in the README (T5) | See §7 for gaps |
| E42 | The Windows hook pauses during a Settings rebind (`set_rebinding` IPC) (T2) | While rebinding, taken combos do the Windows default |
| E43 | `LATER` gate removed so recording works on Windows/macOS (T4) | None |

## 6. Accepted residuals and parked items

These were marked accepted, parked, deferred or "→ Plan N". Items later fixed are listed at the end of this section. The rest had no later fix in the ledgers or final-fixes files at the time of writing. Where the code was spot-checked, the line says so.

**Left open by the Plan 4 final fix wave.** Its 13 items all landed (`b2d7d5e`..`a24d7d2`; see "Later fixed" at the end). What its report leaves open:
- macOS, lost config: the copy in `io.github.kecsou.rshot` is deleted only by a restore. After a lost config, every Quit re-applies it, which reverts a later hand edit of ⌘⇧3/4/5. After a lost config followed by "Not now", the keys stay off for that session.
- A config.toml written before `39d8ffe` (with backups but no defaults copy) gets the copy only the next time its backup changes. Plan 4 is unreleased, so no user is affected.
- The `.deb` long description is one ~290-character line. dpkg and apt accept it; lintian would prefer lines under 80 characters.
- The exit paths, the backup rule and the NSIS pre-install/pre-uninstall hooks are type-checked and unit-tested only. makensis isn't installed here, so the NSIS hooks have never been compiled.
- `a24d7d2` (N1–N3) has no subagent review and no unit test (OS-boundary cfg branches only).

**Plan 1:**
- `package.json` still has npm-init fields (directories, repository, bugs, homepage). Cosmetic.
- Unused Windows-Store icons (`Square*Logo`, `StoreLogo`) still in `src-tauri/icons`. Bigger repo, no runtime effect.
- `write_atomic`: a partial temp file is left if `fs::write` fails; no fsync. A crash or power loss can lose the newest config/capture.
- A symlinked config.toml is replaced by a regular file on save. Dotfile managers lose the link.
- No assert on the `clipboard_mode` wire strings. A rename would silently reset the mode.
- Tests leave `/tmp/rshot-*-<pid>` directories. Temp clutter.
- `dirs` 7 alongside Tauri's `dirs` 6 (Cargo.lock has dirs 4, 6 and 7). Build size.
- A failed move-aside of an invalid config is ignored, and a later bad config overwrites the earlier `.invalid`. One broken config can be lost.
- The clipboard context is created once and never retried. If it fails at startup, copying stays broken until restart.
- No test of the `contents()`/`image_only` mapping. A regression would go unnoticed.
- On Windows/macOS, `copy_image` with an invalid PNG clears the clipboard and returns Ok. The failure is invisible.
- Screen capture grabs every monitor and keeps one. Wasted time on multi-monitor setups.
- Two x11rb versions (0.13 via clipboard-rs, 0.14). Build size.
- The async `overlay_cancel` fix was verified only with synthetic zero-duration keys. A held Esc is untested.
- `overlay_index` asserts sit inside the Target-parsing test. Test hygiene.
- The mallopt SAFETY comment says "before any other thread exists", but the D-Bus forward (zbus threads) now runs first (spot-checked in main.rs). A misleading safety comment.
- `grab_all` stops at the first Err, so a later thread's panic re-panics in the scope (spot-checked). A rare daemon crash on a multi-monitor grab error.
- The D-Bus forwarding path lacks the plugin's `-` → `_` replace. Fine for the current identifier; breaks if it gains a dash.
- A wedged daemon hangs the forwarding CLI (as the plugin does). The shortcut does nothing until the daemon is killed.
- "overlay visible" is logged before focus/map, so the measured latencies are slightly optimistic.
- No "inside" zone for selections under ~16 CSS px (spot-checked): dragging resizes instead of moving.
- The loupe freezes at the edge when the pointer leaves the active monitor.
- `selection.ts` duplicates the `Rect` type from `ipc.ts` (spot-checked). Code hygiene.
- No resize/move cursors over the selection handles (spot-checked: no cursor CSS). A discoverability gap.
- Print, then Space within ~500 ms: a non-active overlay's load can overwrite the broadcast mode.
- The folder dialog is transient but not modal (no `set_modal`, spot-checked). The overlay stays clickable behind it.
- The folder-dialog adoption depends on rfd internals with a silent 3 s ceiling. The dialog may open unfocused after an rfd update.
- The Save-to path ellipsizes at the end, not the start. Long paths hide the folder name.
- A failed `setOverlayOptions` is an unhandled rejection. The option silently doesn't save.
- `pick_folder` runs `blocking_pick_folder` on a tokio worker (spot-checked). One worker is blocked while the dialog is open.
- The drag plugin's `start_drag` accepts any path. Needs a real user drop; low risk.
- Swallowed errors on open/reveal/delete, and no notice when the thumbnail fails AND the copy failed. Silent failures.
- The failure state of the card keeps the check icon (only the colour turns red). A misleading icon.
- The swipe lags because of a CSS transition during the drag. Feel.
- A transparent click-blocking margin around the card (window larger than the card). Clicks just outside the card are eaten.
- If `show_thumbnail` fails before the popup, the previous card stays with dead actions until it auto-dismisses.
- `manual_commands` doubles `%` even for shell-splitting launchers. Negligible.
- Restore clears `gnome_backup` in memory before the media-keys list update; if that update fails, the takeover flag can disagree with reality.
- Autostart/save errors are reported through `takeover_error` (wrong advice; they overwrite each other).
- Onboarding ignores an autostart `enable()` error. "Launch at login" can silently not be set.
- No duplicate-combo check when rebinding. Two actions can share a shortcut.
- Settings `takeover_error` rendering was never exercised at runtime ("Not now" was, in Plan 3).
- `update()` in Settings is two IPC calls, so there's a tiny lost-update window.
- A `save_config` failure overwrites the takeover/rollback error in `set_settings`.
- `scripts/cross-check.sh` fails confusingly when no `llvm-rc` is installed.
- ci.yml runs on both push and pull_request, so PR branches run twice (spot-checked). CI minutes.
- The actions are now SHA-pinned (Plan 4 final #4), but still on the v4 majors (`checkout` and `setup-node` v4.4.0) that the Plan 1 T12 note called Node 20 actions; the major-version bump wasn't made. GitHub will warn when the Node 20 runtime goes away.
- The first CI run (`35865046332`) failed on ubuntu-22.04 (PipeWire headers too old for `libspa` 0.10.1); Linux now builds on ubuntu-24.04 (§1 row 23).
- After uninstall and reinstall, onboarding doesn't reappear (config keeps `onboarded = true`). A product decision left to you.
- A focused always-on-top window keeps focus away from the overlay. Esc/Enter may go to that window.
- A dead `purge` case survives in the shared prerm (dpkg calls postrm for purge). Harmless.
- The `.deb` still Depends on libpipewire, but the release binary doesn't link it (objdump, spot-checked). An unneeded dependency.
- `restore_from_cli` returns early on a missing config, which leaves rshot's custom keybindings if the config was deleted while the takeover was active. Narrow.
- A Settings window open during a forwarded restore can re-take the shortcuts on its next change, and shows the stale toggle until reopened.
- Key → visible is +150 ms (307–357 ms) while a thumbnail card is up.
- 4K and scale-2 save+clipboard timing weren't measured (only 2557×1597: 33–73 ms).
- Restore resets a `[]` you set on purpose to the GNOME default (Plan 1 final fix report, concern 5).

**Plan 2:**
- Check-then-insert race in `ui::open_editor` (spot-checked, still there). Two fast opens of the same file can create two editors.
- Parked cosmetics: outline `measureText` overhang, `pad()` dedupe, rect shadow, box-filter downscale.
- Pixelate flattens thin text, and WebKit sampling makes the blocks harsh (cosmetic; unrecoverable either way).
- Save/Delete act on the canonical file. Deleting through a symlink to a single file removes the target, not the link.
- Close is ignored while a prompt is up; the keyboard-layout fallback applies only to non-Latin layouts; Ctrl+C in the text box runs Copy unless text is selected.
- Never exercised at runtime: the "Saved, but couldn't copy" prompt (item 2) and the non-PNG body refusal (item 18).

**Plan 3:**
- Killing ffmpeg with SIGKILL loses ~2 s of video; a hard-killed rshot loses the MKV tail; a run under ~1.5 s may leave an empty MKV.
- A configured mic that's missing falls back to the default mic. You may record the wrong microphone without a warning.
- A 200 ms race after the countdown (T3 minor 5, same as the timed screenshot path).
- X shape region (T3 minor 7, cosmetic).
- The pill has no keyboard focus (stop with the tray or the shortcut).
- The stream token comes from `RandomState` (accepted with a ponytail note).
- A local process can hold the 32 stream slots. Playback DoS only.
- No mic icon in the native mic `<select>`.
- Overlapping grabs are handled by design; a pill created mid-grab can flash briefly (not in the grab).
- A header double-click didn't maximize under xdo (pre-existing drag.js behaviour; untested with a real mouse).
- Label-wrapped switch rows ("Mute audio", "Drop shadow") may keep focus after a click on the label text.
- Windows: renaming the trimmed temp file over a video the stream thread holds open, and `.trim`/`.Recording_` temp files not being hidden on Windows (dot-files aren't hidden there). These were noted for Plan 4 in the P3 T4 ledger and then dropped; since `a24d7d2` both are in the README Windows smoke list. No code change: a trim may still fail on Windows until it's tested, and the temp files show in Explorer (cosmetic).
- Recording tests: one test recording ran ~70 s (a terminal area) and was discarded. No file remains.

**Plan 4:**
- The Windows hook is installed even when the takeover is declined (accepted T2 minor 7).
- A macOS global-shortcut registration failure leaves the toggle ON until the next launch retries.
- macOS has no Job-object equivalent: a killed rshot leaves ffmpeg recording until the disk fills (ponytail ceiling).
- `hardenedRuntime: false`; TCC grants reset after every update.
- Licensing gaps deferred to you (§1 row 10).
- The rpm's openh264 recommendation is unverified.
- `NSAllowsLocalNetworking` may not cover loopback on macOS 26.5; the fallback is `NSAllowsArbitraryLoads`.
- WebView2 Local Network Access may later block the trimmer's stream.
- Windows Settings can't record `Win+…` or `PrtScn` combos (edit config.toml instead).
- Windows `Ctrl+Alt+<letter>` swallows AltGr+letter (e.g. `€` on AZERTY).
- macOS restore puts ⌘⇧3/4/5 back with default parameters, so a remap you had made is lost.
- macOS letter shortcuts bind by the current layout; restart rshot after switching layouts.

**Later fixed (checked, so not listed above):** `~`/relative screenshots folder (P1 final); `pending` left set after a countdown failure, and a new overlay mid-countdown (P1 final #1); a stale daemon `takeover` after a standalone restore (P1 final #2); the unreachable RestoreShortcuts arm (moot after #2); restored keys becoming explicit dconf values (P3 T5); `remember()` before clamp (P3 T3); per-monitor popover options overwriting each other (P3 T5); the WebKit context menu (P1 T11 `shared/base`); the "15 MB" comment (now MiB); rustfmt drift (P1 T12); the Linux `set_focus` no-op (P1 T6 `force_focus`); the non-Linux clipboard compile (P1 T12 cross-check); the README logged-out custom keybindings (P1 T13); CSP and allow-lists (P3 T0); the Windows `dir_setting` test (`979949b`); the 200 ms countdown sleep blocking a worker (P3 final #4); the redundant `create_dir_all` (function replaced by `save_screenshot`); `core:default` breadth (P3 final #9); the OR pill/frame above an overlay, ENDING in `unavailable()` and the toggle-label focus (P3 final); Plan 3 → Plan 4 licensing carries (P4 T5: `LICENSE`, `deb/copyright`, ubuntu-22.04 release job; partly deferred to you). The Plan 4 final fix wave fixed all 13 of its items:
- #1 (`b2d7d5e`): ⌘Q, logout and Windows sign-out/shutdown release the keys and stop ffmpeg. This also covers the accepted P4 T2 minor 10 (logoff skipped the release).
- #2 (`39d8ffe`): trust the observed value, the macOS defaults copy, the scoped `plutil` read, and backups kept when the hotkeys can't be read.
- #3 and N-residuals (`4649aa5`, `a24d7d2`): the README smoke list.
- #4 (`4649aa5`): SHA pins and `persist-credentials: false`.
- #5: the stale macOS mic comments.
- #6: the NSIS pre-install hook.
- #7 (`5b31ad3`): the macOS permission step after "Not now" too.
- #8: the UIPI note in the README.
- #9: the tray tooltip is gated off Linux.
- #10: `bundle.longDescription`, so the Description no longer ends in "(none)".
- #11: the owner section moved out of THIRD_PARTY.md.
- #12: Settings recorder digits come from `e.code`.
- #13: `to_gnome_accel` enforces the takeable rule.

Also: `b51f0e2` and `c065624` were reviewed OK in the Plan 4 fix-wave re-review, and `a24d7d2` added the macOS CLI restore without config, one release per Quit, and a poison-safe exit lock.

## 7. What only you can verify

**Windows/macOS owner smoke test.** Follow README → "Owner smoke test (Windows and macOS)" plus spec §6. At HEAD `a24d7d2` that list covers every smoke item the ledgers name. The Windows UIPI limitation is under Known limitations. The ledgers also mention these, which the README list doesn't have:
- Windows: with `PrintScreenKeyForSnippingEnabled` already 0 (PrtScn not opening Snipping) before the takeover, it's still off after turning the takeover off (trust the observed value, `39d8ffe`).
- macOS: with config.toml deleted while the keys are taken, `/Applications/rshot.app/Contents/MacOS/rshot restore-shortcuts` brings ⌘⇧3/4/5 back from the defaults copy (`a24d7d2`, N2).
- macOS: `kill -9` of rshot mid-recording leaves ffmpeg recording (known ceiling, no Job object). Check that it's visible and stoppable (Activity Monitor).
- macOS: if the trimmer can't play on macOS 26.5+, `NSAllowsLocalNetworking` isn't enough; the fallback is `NSAllowsArbitraryLoads` alone.

**Linux physical-key checks.** With a real keyboard and the takeover on, press **Shift+Print** (capture the monitor) and **Alt+Print** (capture the window). XTest injection didn't trigger rshot's custom bindings for these two, though plain Print and Ctrl+Alt+Shift+R did. The binding values are correct (`'<Shift>Print'`, `'<Alt>Print'`), so this may be an XTest-only effect.
Also never exercised on Linux: the "Copy failed" + Retry card, the editor's "Saved, but couldn't copy" prompt, a real drag-and-drop from the card into another app, and Settings' `takeover_error` rendering.

**Release workflow.** Run `.github/workflows/release.yml` once from the Actions tab (`workflow_dispatch`) before the first `v*` tag. It exercises the NSIS hooks (pre-install and pre-uninstall; makensis isn't installed here, so they have never been compiled), `lipo` and the dmg, the Git Bash tools (`unzip`, `shasum`) and the AppImage on Ubuntu 24.04 (with `APPIMAGE_EXTRACT_AND_RUN=1`, never run on a runner). The first CI run passed on Windows and macOS and failed on ubuntu-22.04; after this fix, check that CI passed on all three OSes (ubuntu-24.04, windows-latest, macos-latest).

**Unsigned builds.** Windows SmartScreen warns (More info → Run anyway). macOS Gatekeeper blocks the ad-hoc-signed app until `xattr -dr com.apple.quarantine /Applications/rshot.app`. The Screen Recording and Microphone grants must be given again after every update.

## 8. Leftovers on this machine

- **Two apport crash dialogs and `/var/crash/_usr_bin_python3.12.1000.crash`** (04:30, 11 MB; still there when this was written). They come from a reviewer's offline WebKitGTK probe (`python3 probe.py probe.html`). The permission classifier denied the controller's attempt to close or remove them, so dismiss the dialogs and delete the file yourself.
- **GNOME screenshot keys have no dconf user value** (checked: `dconf read` prints nothing for show-screenshot-ui, screenshot, screenshot-window, show-screen-recording-ui and media-keys custom-keybindings). They're at the GNOME 46 defaults (`['Print']`, `['<Shift>Print']`, `['<Alt>Print']`, `['<Ctrl><Shift><Alt>R']`, `@as []`). Before the run they may have held explicit values equal to those defaults; no dconf dump was taken, so the original user-value state is unknown. Functionally the same.
- **The clipboard may hold a test file path** from the last test capture (Plans 2 and 3 reports).
- GNOME notifications from the Plan 3 tests (recovered and couldn't-recover messages, saved captures) may still sit in the notification list.
- `src-tauri/target/release/rshot` and the bundles in `target/release/bundle/` were built at `4649aa5` (14:54). `a24d7d2` changed only README and Windows/macOS code paths after that. Rebuild before any release-mode check.
- Locally, `npm run tauri build` fails at the AppImage step because `patchelf` isn't installed (no sudo). The fix wave used a patchelf from the session scratchpad. CI installs it.
- Git-ignored working files kept on purpose: `.superpowers/sdd/` (these ledgers) and `src-tauri/binaries/` (the fetched ffmpeg sidecar and licence).
- Noticed, not caused by the run: an NVIDIA driver/library mismatch pending a reboot (hence rshot's `WEBKIT_DISABLE_DMABUF_RENDERER=1` default), and a root-owned file in `~/.cargo/registry` that triggers cargo's auto-clean warning.
