# Graph Report - rshot  (2026-09-30)

## Corpus Check
- 72 files · ~122,007 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1445 nodes · 2895 edges · 70 communities (67 shown, 3 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 112 edges (avg confidence: 0.85)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `d74da466`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- [[_COMMUNITY_Recorder Backends|Recorder Backends]]
- [[_COMMUNITY_Overlay Windows & Focus|Overlay Windows & Focus]]
- [[_COMMUNITY_Config Store|Config Store]]
- [[_COMMUNITY_Overlay Frontend & Latency|Overlay Frontend & Latency]]
- [[_COMMUNITY_macOS Shortcut Takeover|macOS Shortcut Takeover]]
- [[_COMMUNITY_Frontend IPC Wrappers|Frontend IPC Wrappers]]
- [[_COMMUNITY_Tauri App Config|Tauri App Config]]
- [[_COMMUNITY_Windows Keyboard Hook|Windows Keyboard Hook]]
- [[_COMMUNITY_Overlay Capture Commands|Overlay Capture Commands]]
- [[_COMMUNITY_GNOME Shortcut Takeover|GNOME Shortcut Takeover]]
- [[_COMMUNITY_Editor Shape Model|Editor Shape Model]]
- [[_COMMUNITY_Editor Canvas Main|Editor Canvas Main]]
- [[_COMMUNITY_Screen Capture & Crop|Screen Capture & Crop]]
- [[_COMMUNITY_Loopback Video Server|Loopback Video Server]]
- [[_COMMUNITY_Memory & Runtime Tuning|Memory & Runtime Tuning]]
- [[_COMMUNITY_Recording Flow Design|Recording Flow Design]]
- [[_COMMUNITY_Clipboard Owner Thread|Clipboard Owner Thread]]
- [[_COMMUNITY_NPM Package Manifest|NPM Package Manifest]]
- [[_COMMUNITY_Editor Actions & Crop|Editor Actions & Crop]]
- [[_COMMUNITY_Thumbnail Path Guard|Thumbnail Path Guard]]
- [[_COMMUNITY_Design Spec & Perf Targets|Design Spec & Perf Targets]]
- [[_COMMUNITY_Bundled ffmpeg & Release|Bundled ffmpeg & Release]]
- [[_COMMUNITY_Visual Direction Mockups|Visual Direction Mockups]]
- [[_COMMUNITY_Settings Backend|Settings Backend]]
- [[_COMMUNITY_Editor Rendering & Redact|Editor Rendering & Redact]]
- [[_COMMUNITY_Onboarding & Settings UI|Onboarding & Settings UI]]
- [[_COMMUNITY_Cross-Platform Plan 4|Cross-Platform Plan 4]]
- [[_COMMUNITY_Plan 1-2 Frontend Architecture|Plan 1-2 Frontend Architecture]]
- [[_COMMUNITY_Shortcut Commands Core|Shortcut Commands Core]]
- [[_COMMUNITY_Small Window Frontends|Small Window Frontends]]
- [[_COMMUNITY_Shortcut Combo Parser|Shortcut Combo Parser]]
- [[_COMMUNITY_Editor Backend Commands|Editor Backend Commands]]
- [[_COMMUNITY_Video Trim UI|Video Trim UI]]
- [[_COMMUNITY_Plan 1 Foundation|Plan 1 Foundation]]
- [[_COMMUNITY_Capture Pipeline|Capture Pipeline]]
- [[_COMMUNITY_Windows Bundle Config|Windows Bundle Config]]
- [[_COMMUNITY_Cross-Check Script|Cross-Check Script]]
- [[_COMMUNITY_App State & Editors|App State & Editors]]
- [[_COMMUNITY_Capture & Clipboard Plan|Capture & Clipboard Plan]]
- [[_COMMUNITY_macOS Bundle Config|macOS Bundle Config]]
- [[_COMMUNITY_TypeScript Config|TypeScript Config]]
- [[_COMMUNITY_Settings & Onboarding Plan|Settings & Onboarding Plan]]
- [[_COMMUNITY_Shortcut Recorder Keys|Shortcut Recorder Keys]]
- [[_COMMUNITY_Thumbnail & Trim Helpers|Thumbnail & Trim Helpers]]
- [[_COMMUNITY_Undo History|Undo History]]
- [[_COMMUNITY_Linux Bundle Config|Linux Bundle Config]]
- [[_COMMUNITY_X11 Test Driver|X11 Test Driver]]
- [[_COMMUNITY_Countdown Capability|Countdown Capability]]
- [[_COMMUNITY_Default Capability|Default Capability]]
- [[_COMMUNITY_Editor Capability|Editor Capability]]
- [[_COMMUNITY_Onboarding Capability|Onboarding Capability]]
- [[_COMMUNITY_Overlay Capability|Overlay Capability]]
- [[_COMMUNITY_Pill Capability|Pill Capability]]
- [[_COMMUNITY_Settings Capability|Settings Capability]]
- [[_COMMUNITY_Thumbnail Capability|Thumbnail Capability]]
- [[_COMMUNITY_Deb Pre-Remove Hook|Deb Pre-Remove Hook]]
- [[_COMMUNITY_Vite Config|Vite Config]]
- [[_COMMUNITY_Community 58|Community 58]]
- [[_COMMUNITY_Community 59|Community 59]]
- [[_COMMUNITY_Community 60|Community 60]]
- [[_COMMUNITY_Community 61|Community 61]]
- [[_COMMUNITY_Community 62|Community 62]]
- [[_COMMUNITY_Community 63|Community 63]]
- [[_COMMUNITY_Community 64|Community 64]]
- [[_COMMUNITY_Community 65|Community 65]]
- [[_COMMUNITY_Community 66|Community 66]]
- [[_COMMUNITY_Community 67|Community 67]]
- [[_COMMUNITY_Community 68|Community 68]]
- [[_COMMUNITY_Community 69|Community 69]]

## God Nodes (most connected - your core abstractions)
1. `$()` - 68 edges
2. `$()` - 34 edges
3. `AppHandle` - 31 edges
4. `Result` - 24 edges
5. `String` - 21 edges
6. `String` - 21 edges
7. `Config` - 18 edges
8. `PathBuf` - 18 edges
9. `render()` - 18 edges
10. `String` - 17 edges

## Surprising Connections (you probably didn't know these)
- `trim_video()` --implements--> `Video trim editor (spec 2.7)`  [INFERRED]
  src-tauri/src/editor.rs → docs/superpowers/specs/2026-09-22-rshot-design.md
- `Non-active overlays wait for overlay:primary-ready` --references--> `overlay_ready()`  [INFERRED]
  docs/perf.md → src-tauri/src/overlay.rs
- `record_args()` --implements--> `Pure no-I/O functions for unit tests`  [INFERRED]
  src-tauri/src/recorder.rs → docs/superpowers/specs/2026-09-22-rshot-design.md
- `remux_args()` --implements--> `Crash-safe MKV capture remuxed to MP4`  [INFERRED]
  src-tauri/src/recorder.rs → docs/superpowers/specs/2026-09-22-rshot-design.md
- `restore_from_cli()` --implements--> `Quit/uninstall release keys but keep consent (--keep-consent)`  [INFERRED]
  src-tauri/src/shortcuts/mod.rs → docs/superpowers/2026-09-23-autonomous-run-decisions.md

## Import Cycles
- 1-file cycle: `src-tauri/src/ui.rs -> src-tauri/src/ui.rs`
- 1-file cycle: `src-tauri/src/clipboard.rs -> src-tauri/src/clipboard.rs`
- 1-file cycle: `src-tauri/src/editor.rs -> src-tauri/src/editor.rs`
- 1-file cycle: `src-tauri/src/pipeline.rs -> src-tauri/src/pipeline.rs`
- 1-file cycle: `src-tauri/src/shortcuts/gnome.rs -> src-tauri/src/shortcuts/gnome.rs`
- 1-file cycle: `src-tauri/src/shortcuts/macos.rs -> src-tauri/src/shortcuts/macos.rs`
- 1-file cycle: `src-tauri/src/shortcuts/windows.rs -> src-tauri/src/shortcuts/windows.rs`
- 1-file cycle: `src-tauri/src/thumbnail.rs -> src-tauri/src/thumbnail.rs`

## Hyperedges (group relationships)
- **Capture-to-clipboard pipeline (grab, name, atomic write, clipboard, shutter, thumbnail)** — plans_2026_09_22_plan_1_foundation_linux_screenshots_capture_module, plans_2026_09_22_plan_1_foundation_linux_screenshots_finish_capture, plans_2026_09_22_plan_1_foundation_linux_screenshots_capture_naming, plans_2026_09_22_plan_1_foundation_linux_screenshots_atomic_write, plans_2026_09_22_plan_1_foundation_linux_screenshots_clipboard_owner_thread, plans_2026_09_22_plan_1_foundation_linux_screenshots_play_shutter, plans_2026_09_22_plan_1_foundation_linux_screenshots_floating_thumbnail [EXTRACTED 1.00]
- **GNOME shortcut takeover/restore lifecycle** — plans_2026_09_22_plan_1_foundation_linux_screenshots_gnome_shortcut_takeover, plans_2026_09_22_plan_1_foundation_linux_screenshots_onboarding, plans_2026_09_22_plan_1_foundation_linux_screenshots_settings_window, plans_2026_09_22_plan_1_foundation_linux_screenshots_restore_shortcuts_cli, plans_2026_09_22_plan_1_foundation_linux_screenshots_prerm_restore, plans_2026_09_22_plan_1_foundation_linux_screenshots_store_config [INFERRED 0.85]
- **Editor frontend split: model / render / styles / main** — plans_2026_09_22_plan_2_image_editor_model_ts, plans_2026_09_22_plan_2_image_editor_render_ts, plans_2026_09_22_plan_2_image_editor_styles_ts, plans_2026_09_22_plan_2_image_editor_editor_main_ts [EXTRACTED 1.00]
- **Recording lifecycle: countdown, ffmpeg MKV, watchdog, stop, remux, finish** — specs_2026_09_22_rshot_design_recording_behaviour, plans_2026_09_22_plan_3_recording_record_countdown, src_recorder_start, plans_2026_09_22_plan_3_recording_recording_watchdog, plans_2026_09_22_plan_3_recording_graceful_ffmpeg_stop, src_recorder_remux_args, src_pipeline_finish_video [EXTRACTED 1.00]
- **Per-OS shortcut takeover and restore** — specs_2026_09_22_rshot_design_shortcut_takeover, shortcuts_gnome, shortcuts_windows, shortcuts_macos, src_combo_takeable, superpowers_2026_09_23_autonomous_run_decisions_keep_consent_release, superpowers_2026_09_23_autonomous_run_decisions_trust_observed_value [INFERRED 0.85]
- **Getting recorded video into the webview safely** — plans_2026_09_22_plan_3_recording_asset_protocol_video, superpowers_2026_09_23_autonomous_run_decisions_loopback_range_server, src_stream, superpowers_2026_09_23_autonomous_run_decisions_poster_thumbnail, plans_2026_09_22_plan_3_recording_webview_hardening [INFERRED 0.85]
- **Screen recording UI flow (countdown, frame, pill, thumbnail, trimmer)** — readme_screen_recording, countdown_index, recframe_index, pill_index, thumbnail_index, video_index [INFERRED 0.85]
- **Per-platform shortcut takeover mechanisms** — readme_shortcut_takeover, readme_gnome_custom_keybindings, readme_windows_keyboard_hook, readme_macos_symbolic_hotkeys, readme_restore_shortcuts [INFERRED 0.85]
- **Bundled GPL ffmpeg compliance chain** — readme_rshot_ffmpeg_sidecar, third_party_btbn_ffmpeg_builds, third_party_martin_riedl_builds, third_party_x264_0480cb05, third_party_ffmpeg_notices, workflows_release_create_release, readme_licensing_gaps [INFERRED 0.85]

## Communities (70 total, 3 thin omitted)

### Community 0 - "Recorder Backends"
Cohesion: 0.07
Nodes (75): Arc, AtomicU32, Child, Command, Device, Duration, Output, gdigrab/dshow and avfoundation recording backends (+67 more)

### Community 1 - "Overlay Windows & Focus"
Cohesion: 0.09
Nodes (69): Mutter focus-stealing prevention (stale _NET_WM_USER_TIME), On-demand overlays experiment (reverted), Preloaded overlay windows, Fn, Hidden, Image, Position, Size (+61 more)

### Community 2 - "Config Store"
Cohesion: 0.10
Nodes (52): Default, Iterator, NaiveDateTime, absolute(), bad_config_is_moved_aside_not_lost(), capture_stem(), ClipboardMode, Config (+44 more)

### Community 3 - "Overlay Frontend & Latency"
Cohesion: 0.07
Nodes (51): 16 MB frame IPC + putImageData bottleneck (~117 ms), key -> visible overlay latency target (<= 250 ms), Non-active overlays wait for overlay:primary-ready, argv forwarding over single-instance D-Bus before GTK/Tauri start, Magnifier loupe (pixel grid, coords, colour), Overlay options popover (timer, remember selection, pointer, mic), Window mode (Space, highlighted window), overlay/index.html (+43 more)

### Community 4 - "macOS Shortcut Takeover"
Cohesion: 0.10
Nodes (45): Combo, FnOnce, Key, backup(), backup_key(), backups(), bad_and_bare_combos_are_refused(), bindings() (+37 more)

### Community 5 - "Frontend IPC Wrappers"
Cohesion: 0.04
Nodes (13): BoolSetting, ClipboardMode, EditorInfo, Mic, OverlayInfo, OverlayOptions, RecordingInfo, Rect (+5 more)

### Community 6 - "Tauri App Config"
Cohesion: 0.04
Nodes (45): app, macOSPrivateApi, security, windows, bundleMediaFramework, build, beforeBuildCommand, beforeDevCommand (+37 more)

### Community 7 - "Windows Keyboard Hook"
Cohesion: 0.10
Nodes (40): Binding, LPARAM, LRESULT, Mods, a_press_long_after_the_last_event_is_new_not_a_repeat(), a_taken_key_runs_once_and_its_repeats_and_release_are_swallowed(), backup(), bindings() (+32 more)

### Community 8 - "Overlay Capture Commands"
Cohesion: 0.14
Nodes (39): after_countdown(), capture_from(), countdown_cancel(), countdown_done(), countdown_info(), overlay_activate(), overlay_cancel(), overlay_capture() (+31 more)

### Community 9 - "GNOME Shortcut Takeover"
Cohesion: 0.11
Nodes (26): differs(), gs(), gsettings_ok(), our_paths(), outdated(), path(), Put, put_back() (+18 more)

### Community 10 - "Editor Shape Model"
Cohesion: 0.08
Nodes (28): Base, bounds(), cropFromDrag(), distToSeg(), dragHandle(), Ends, HandleId, handles() (+20 more)

### Community 11 - "Editor Canvas Main"
Cohesion: 0.10
Nodes (30): $(), adoptStyle(), autosize(), canvas, doc(), Drag, hist, KEYS (+22 more)

### Community 12 - "Screen Capture & Crop"
Cohesion: 0.11
Nodes (30): IntoIterator, clamp_rect(), composite(), crop(), crop_then_encode_gives_a_png_of_that_size(), Cursor, encode_png(), focused_window_image() (+22 more)

### Community 13 - "Loopback Video Server"
Cohesion: 0.12
Nodes (27): Read, accept(), answer(), byte_range(), Deadline, empty(), file_head(), head() (+19 more)

### Community 14 - "Memory & Runtime Tuning"
Cohesion: 0.08
Nodes (27): AtomicU64, Clipboard, Display, malloc_trim(0) after session (dropped), Memory target (<= 320 MB PSS after use), Fixed M_MMAP_THRESHOLD = 1 MB, WEBKIT_DISABLE_DMABUF_RENDERER=1 workaround, HashMap (+19 more)

### Community 15 - "Recording Flow Design"
Cohesion: 0.18
Nodes (18): Mockup 04: screen recording, Dashed red recording frame outside the area, Dashed red recording frame (recframe), Frame-accurate trim (-ss before -i with re-encode), Graceful ffmpeg stop (q on stdin, 10 s deadline, then kill), 3 s record countdown via pending_rec, Recording control pill, Recording watchdog thread (+10 more)

### Community 16 - "Clipboard Owner Thread"
Cohesion: 0.16
Nodes (20): ClipboardContent, Job, Sender, Clipboard, contents(), file_uri(), fits_x11(), formats_carry_path_uri_and_optional_png() (+12 more)

### Community 17 - "NPM Package Manifest"
Cohesion: 0.07
Nodes (26): bugs, url, dependencies, @crabnebula/tauri-plugin-drag, @fontsource/inter, @tauri-apps/api, devDependencies, @tauri-apps/cli (+18 more)

### Community 18 - "Editor Actions & Crop"
Cohesion: 0.30
Nodes (15): commit(), done(), enterCrop(), exitCrop(), fit(), full(), moveCropDrag(), redo() (+7 more)

### Community 19 - "Thumbnail Path Guard"
Cohesion: 0.21
Nodes (22): AppHandle, AppState, Option, Path, PathBuf, Response, Result, State (+14 more)

### Community 20 - "Design Spec & Perf Targets"
Cohesion: 0.10
Nodes (27): Capture -> saved + clipboard latency (<= 300 ms), Simplification: pen/highlighter/text/counter move but don't resize, Vector shapes in image-pixel coordinates until export, Absolute file path on clipboard, Clipboard contract (path + file + image), Floating thumbnail card, Frozen-frame overlay per monitor, Style A - Glass visual contract (+19 more)

### Community 21 - "Bundled ffmpeg & Release"
Cohesion: 0.16
Nodes (18): fetch-ffmpeg.sh script, ffmpeg-notices.sh script, Develop prerequisites (Rust stable, Node 22, apt libs), Linux packages need glibc 2.39 (Ubuntu 24.04 build), Licensing gaps before first public release, rshot-ffmpeg bundled sidecar, Screen recording flow (3 s countdown, MP4 H.264), check() (+10 more)

### Community 22 - "Visual Direction Mockups"
Cohesion: 0.22
Nodes (9): countdown/index.html, Mockup 01 Visual Direction (Glass, thumbnail 01-A), Direction B: Adwaita GNOME-native, Direction A: Glass macOS-like (chosen), Direction C: Signal developer tool (monospace, lime), Timer countdown ring, Recording controls pill, pill/index.html (+1 more)

### Community 23 - "Settings Backend"
Cohesion: 0.20
Nodes (20): catch_up_takeover(), close_window(), get_settings(), onboarding_choice(), open_config(), set_settings(), Settings, snapshot() (+12 more)

### Community 24 - "Editor Rendering & Redact"
Cohesion: 0.16
Nodes (21): Doc, exportOrder(), Rect, redactBlock(), redactRect(), Shape, strokeWidth(), TEXT_PX (+13 more)

### Community 25 - "Onboarding & Settings UI"
Cohesion: 0.12
Nodes (18): Mockup 02 Overlay States, First-launch takeover prompt, Settings as one scrolling page, no tabs, Native tray menu, onboarding/index.html, icons.ts SVG sprite (SPRITE, mountIcons), Editor icons added to SPRITE, GNOME gsettings/dconf custom keybindings (+10 more)

### Community 26 - "Cross-Platform Plan 4"
Cohesion: 0.15
Nodes (18): Record shortcut takeover (show-screen-recording-ui -> rshot record), Platform differences behind #[cfg(target_os)] in owning modules, WH_KEYBOARD_LL hook with 0xE8 dummy key, Neutral shortcut combo parser (combo.rs), NSIS installer hooks (hooks.nsh), Per-OS default shortcuts table, macOS simple-fullscreen overlays, macOS symbolic hotkeys + global-shortcut plugin (+10 more)

### Community 27 - "Plan 1-2 Frontend Architecture"
Cohesion: 0.16
Nodes (19): crate::err (IPC errors as String), src/shared/ipc.ts typed invoke wrappers, overlay::Target (area | window | screen), selection.ts geometry (fromPoints, clamp, move, resize, handleAt, windowAt), Vite multi-page build (src/<page> -> dist/<page>), Crop with presets Free/16:9/4:3/1:1 (cropFromDrag, fitRatio), Editor interaction main.ts (tools, selection, text, crop, zoom, save/close), Editor tools and keys (V C A R O L P H T N B) (+11 more)

### Community 28 - "Shortcut Commands Core"
Cohesion: 0.23
Nodes (16): command_for(), exe_command(), live_path(), manual_commands(), outdated(), release(), restore(), restore_and_save() (+8 more)

### Community 29 - "Small Window Frontends"
Cohesion: 0.19
Nodes (8): timer, next(), permissionStep(), rows, mountIcons(), run(), setCopied(), clock()

### Community 30 - "Shortcut Combo Parser"
Cohesion: 0.23
Nodes (13): bindable(), c(), Combo, Key, parse(), parse_key(), takeable(), takeable_needs_ctrl_alt_or_super_unless_print() (+5 more)

### Community 31 - "Editor Backend Commands"
Cohesion: 0.33
Nodes (17): Request, copy_path(), editor_delete(), editor_info(), editor_paths(), EditorInfo, open_editor(), save_image() (+9 more)

### Community 32 - "Video Trim UI"
Cohesion: 0.16
Nodes (18): usKey(), $(), changed(), copy(), done(), probe, render(), requestClose() (+10 more)

### Community 33 - "Plan 1 Foundation"
Cohesion: 0.17
Nodes (15): Autonomous run environment notes (no sudo, xdo.py), cli::Cmd / cli::parse, Debian package (.deb), self-sufficient, crate::dispatch(&AppHandle, Cmd), ui::force_focus (X server timestamp present), Plan-1 manual checklist, overlay::start / overlay::Session, Performance and focus gate (spec §5, docs/perf.md) (+7 more)

### Community 34 - "Capture Pipeline"
Cohesion: 0.35
Nodes (14): capture_screen_now(), capture_window_now(), finish(), finish_capture(), finish_video(), notify(), play_shutter(), AppHandle (+6 more)

### Community 35 - "Windows Bundle Config"
Cohesion: 0.14
Nodes (13): bundle, externalBin, resources, targets, windows, installerHooks, installMode, binaries/FFMPEG-LICENSE.txt (+5 more)

### Community 36 - "Cross-Check Script"
Cohesion: 0.17
Nodes (11): cross-check.sh script, Cross-check verification (clippy for Windows/macOS targets), CARGO_TARGET_DIR, CC_aarch64_apple_darwin, CC_x86_64_apple_darwin, DOCS_RS, PATH, TAURI_CONFIG (+3 more)

### Community 37 - "App State & Editors"
Cohesion: 0.27
Nodes (12): AppState (config, clipboard, last_capture, session, pending, thumb), Floating thumbnail (thumbnail.rs, Thumb), ui::monitor_at, ui::popup / close_prefix / POPUP_SEQ, thumbnail::guard path guard, Timer countdown window (overlay::Pending), AppState.editors (label -> path map), editor.rs commands (open_editor, editor_info, save_image, editor_delete, copy_path) (+4 more)

### Community 38 - "Capture & Clipboard Plan"
Cohesion: 0.18
Nodes (15): store::write_atomic (temp file + rename), capture.rs (grab_all, frame_at, clamp_rect, crop, encode_png, windows_on, window_image), Capture naming Screenshot_YYYY-MM-DD_HH-MM-SS.png (unique_path, new_screenshot_path), CI workflow Linux/Windows/macOS (ci.yml), clipboard::Clipboard owner thread (copy_capture, copy_image), Clipboard payload: path + image vs path only, clipboard-rs 0.3 crate, pipeline::finish_capture (+7 more)

### Community 39 - "macOS Bundle Config"
Cohesion: 0.17
Nodes (11): bundle, externalBin, macOS, resources, targets, hardenedRuntime, minimumSystemVersion, signingIdentity (+3 more)

### Community 40 - "TypeScript Config"
Cohesion: 0.17
Nodes (11): compilerOptions, isolatedModules, lib, module, moduleResolution, noEmit, skipLibCheck, strict (+3 more)

### Community 41 - "Settings & Onboarding Plan"
Cohesion: 0.31
Nodes (10): Mockup 05 Settings/Onboarding/Tray, GNOME shortcut takeover/restore (shortcuts::take_over, restore), shortcuts::gvariant (parse_strv, format_strv, to_gnome_accel, with_paths, without_paths), keys.ts comboFrom, Onboarding (Use rshot choice), Overlay options popover (options.ts, set_overlay_options, pick_folder), Settings window (settings.rs get_settings/set_settings), store::Config (TOML, serde default) (+2 more)

### Community 42 - "Shortcut Recorder Keys"
Cohesion: 0.25
Nodes (6): comboFrom(), KeyLike, Mods, esc(), render(), update()

### Community 43 - "Thumbnail & Trim Helpers"
Cohesion: 0.09
Nodes (23): 1. Goals and non-goals, 2.1 Capture modes and default shortcuts, 2.2 Overlay (mockup 02), 2.3 After a capture, 2.4 Clipboard contract, 2.5 Image editor (mockup 03), 2.6 Recording (mockup 04), 2.7 Video editor (mockup 04d) (+15 more)

### Community 45 - "Linux Bundle Config"
Cohesion: 0.25
Nodes (7): bundle, externalBin, resources, targets, binaries/FFMPEG-LICENSE.txt, ../FFMPEG-NOTICES.txt, ../THIRD_PARTY.md

### Community 46 - "X11 Test Driver"
Cohesion: 0.43
Nodes (4): button(), key(), move(), open_display()

### Community 47 - "Countdown Capability"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 48 - "Default Capability"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 49 - "Editor Capability"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 50 - "Onboarding Capability"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 51 - "Overlay Capability"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 52 - "Pill Capability"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 53 - "Settings Capability"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 54 - "Thumbnail Capability"
Cohesion: 0.33
Nodes (5): description, identifier, permissions, $schema, windows

### Community 58 - "Community 58"
Cohesion: 0.11
Nodes (19): Environment notes (the author's machine, read before any task), File Map (end state of this plan), Global Constraints, Prerequisites (the user runs these once, before Task 1), rshot Plan 1: Foundation + Linux Screenshots — Implementation Plan, Self-review notes (for the executor), Task 10: GNOME shortcut takeover and restore, Task 11: Onboarding, Settings, final tray menu (+11 more)

### Community 59 - "Community 59"
Cohesion: 0.16
Nodes (15): rshot-ffmpeg sidecar naming, GitHub Actions release workflow (tauri-action), Owner smoke test checklist (Windows and macOS), Unsigned Windows/macOS builds, Bundled static ffmpeg sidecar (externalBin), Packaging and distribution (deb/rpm/AppImage/NSIS/dmg), Ad-hoc signing with hardenedRuntime off (macOS), Packages recommend GStreamer H.264 codecs for the trimmer (+7 more)

### Community 60 - "Community 60"
Cohesion: 0.14
Nodes (14): A. Preloaded overlays, as built by Task 5 (no fix), B. Preloaded overlays + `force_focus` (brief fallback: fresh X server time + `present_with_time`), C. Experiment, not kept: on-demand overlays (brief's memory fallback), Capture → saved + clipboard (2026-09-23, final-review fix wave), D1. Items 1–4 as specified (`malloc_trim(0)` after each session), D2. Fixed mmap threshold instead (`mallopt(M_MMAP_THRESHOLD, 1 MB)` at daemon start, committed), Decision, round 1, Decision, round 2 (+6 more)

### Community 61 - "Community 61"
Cohesion: 0.36
Nodes (11): commitText(), copy(), fatal(), remove(), requestClose(), save(), $(), ask() (+3 more)

### Community 62 - "Community 62"
Cohesion: 0.18
Nodes (11): File Map, Global Constraints, rshot Plan 3: Screen Recording + Video Trim — Implementation Plan, Self-review notes, Task 0: Webview hardening — CSP and per-window command allow-lists (Plan 1 final-review ruling), Task 1: ffmpeg sidecar + pure recorder functions, Task 2: Recording engine — config, start/stop/discard, watchdog, tray, `rshot record`, Task 3: Recording UI — overlay record modes, countdown, frame and pill (+3 more)

### Community 63 - "Community 63"
Cohesion: 0.24
Nodes (9): $(), bars, discard, mic, pad(), poll(), SHAPE, time (+1 more)

### Community 64 - "Community 64"
Cohesion: 0.22
Nodes (9): File Map, Global Constraints, rshot Plan 4: Windows + macOS Support, Installers, Release — Implementation Plan, Self-review notes, Task 1: Neutral shortcut combos + per-OS defaults, Task 2: Windows shortcut takeover (registry + low-level keyboard hook), Task 3: macOS — shortcut takeover, overlays, tray, permission, sound, Task 4: Recording on Windows and macOS (+1 more)

### Community 65 - "Community 65"
Cohesion: 0.22
Nodes (9): Before the first public release, Develop, Give the shortcuts back, Install, Known limitations, Other Linux desktops, Owner smoke test (Windows and macOS), Record the screen (+1 more)

### Community 66 - "Community 66"
Cohesion: 0.22
Nodes (9): 1. Rulings that change what you asked for or approved, 2. Plan 1: foundation and Linux screenshots, 3. Plan 2: image editor, 4. Plan 3: recording, 5. Plan 4: Windows, macOS, installers, 6. Accepted residuals and parked items, 7. What only you can verify, 8. Leftovers on this machine (+1 more)

### Community 67 - "Community 67"
Cohesion: 0.29
Nodes (8): editor/index.html, Mockup 03 Editor, Redact tool (pixelate/blur/solid, irreversible), Done saves over the same file, Editor tool rail with single-key shortcuts, Video trim editor, Trimmer loopback video server (127.0.0.1), video/index.html

### Community 68 - "Community 68"
Cohesion: 0.25
Nodes (8): File Map, Global Constraints, rshot Plan 2: Image Editor — Implementation Plan, Self-review notes, Task 1: Editor window and commands, thumbnail/tray wiring, save round-trip, Task 2: Document model (`model.ts`), test-first, Task 3: Canvas rendering and export (`render.ts`) + style popover (`styles.ts`), Task 4: Editor interaction (`main.ts`): tools, selection, text, crop, zoom, save/close flows

### Community 69 - "Community 69"
Cohesion: 0.50
Nodes (3): Video delivery via Tauri asset protocol (per-file scope), Webview hardening: CSP + per-window command allow-lists, Loopback HTTP range server for video preview

## Knowledge Gaps
- **386 isolated node(s):** `name`, `version`, `doc`, `test`, `dev` (+381 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **3 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `$()` connect `Editor Canvas Main` to `Video Trim UI`, `Community 67`, `Frontend IPC Wrappers`, `Editor Shape Model`, `Editor Actions & Crop`, `Small Window Frontends`, `Editor Rendering & Redact`, `Community 61`?**
  _High betweenness centrality (0.116) - this node is a cross-community bridge._
- **Why does `Trust the observed value on first takeover (+ macOS defaults-domain backup copy)` connect `Cross-Platform Plan 4` to `macOS Shortcut Takeover`, `Design Spec & Perf Targets`, `Windows Keyboard Hook`?**
  _High betweenness centrality (0.113) - this node is a cross-community bridge._
- **Why does `$()` connect `Community 63` to `Frontend IPC Wrappers`, `Small Window Frontends`, `Visual Direction Mockups`, `Recording Flow Design`?**
  _High betweenness centrality (0.104) - this node is a cross-community bridge._
- **What connects `name`, `version`, `doc` to the rest of the system?**
  _389 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Recorder Backends` be split into smaller, more focused modules?**
  _Cohesion score 0.07434616514839847 - nodes in this community are weakly interconnected._
- **Should `Overlay Windows & Focus` be split into smaller, more focused modules?**
  _Cohesion score 0.08998435054773082 - nodes in this community are weakly interconnected._
- **Should `Config Store` be split into smaller, more focused modules?**
  _Cohesion score 0.09899749373433583 - nodes in this community are weakly interconnected._