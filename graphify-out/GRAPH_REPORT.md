# Graph Report - .  (2026-09-30)

## Corpus Check
- 88 files · ~121,823 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1339 nodes · 2785 edges · 58 communities (55 shown, 3 thin omitted)
- Extraction: 96% EXTRACTED · 4% INFERRED · 0% AMBIGUOUS · INFERRED: 112 edges (avg confidence: 0.85)
- Token cost: 396,097 input · 0 output

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

## God Nodes (most connected - your core abstractions)
1. `$()` - 68 edges
2. `Autonomous run decisions log` - 36 edges
3. `$()` - 34 edges
4. `AppHandle` - 31 edges
5. `rshot Design Spec` - 31 edges
6. `Result` - 24 edges
7. `rshot Plan 3: Recording` - 23 edges
8. `String` - 21 edges
9. `String` - 21 edges
10. `rshot Plan 4: Windows/macOS + Installers` - 21 edges

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

## Communities (58 total, 3 thin omitted)

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
Cohesion: 0.15
Nodes (38): after_countdown(), capture_from(), countdown_cancel(), countdown_done(), countdown_info(), overlay_activate(), overlay_cancel(), overlay_capture() (+30 more)

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
Cohesion: 0.10
Nodes (27): Mockup 04: screen recording, Dashed red recording frame outside the area, rshot Plan 3: Recording, Video delivery via Tauri asset protocol (per-file scope), Dashed red recording frame (recframe), Frame-accurate trim (-ss before -i with re-encode), Graceful ffmpeg stop (q on stdin, 10 s deadline, then kill), 3 s record countdown via pending_rec (+19 more)

### Community 16 - "Clipboard Owner Thread"
Cohesion: 0.16
Nodes (20): ClipboardContent, Job, Sender, Clipboard, contents(), file_uri(), fits_x11(), formats_carry_path_uri_and_optional_png() (+12 more)

### Community 17 - "NPM Package Manifest"
Cohesion: 0.07
Nodes (26): bugs, url, dependencies, @crabnebula/tauri-plugin-drag, @fontsource/inter, @tauri-apps/api, devDependencies, @tauri-apps/cli (+18 more)

### Community 18 - "Editor Actions & Crop"
Cohesion: 0.18
Nodes (26): commit(), commitText(), copy(), done(), enterCrop(), exitCrop(), fatal(), fit() (+18 more)

### Community 19 - "Thumbnail Path Guard"
Cohesion: 0.21
Nodes (22): AppHandle, AppState, Option, Path, PathBuf, Response, Result, State (+14 more)

### Community 20 - "Design Spec & Perf Targets"
Cohesion: 0.13
Nodes (25): docs/perf.md performance gate report, Capture -> saved + clipboard latency (<= 300 ms), rshot Design Spec, Clipboard contract (path + file + image), Floating thumbnail card, Image editor (spec 2.5), Performance targets (spec 5), Preloaded overlay windows (+17 more)

### Community 21 - "Bundled ffmpeg & Release"
Cohesion: 0.12
Nodes (22): fetch-ffmpeg.sh script, ffmpeg-notices.sh script, rshot-ffmpeg sidecar naming, Develop prerequisites (Rust stable, Node 22, apt libs), Linux packages need glibc 2.39 (Ubuntu 24.04 build), Licensing gaps before first public release, Owner smoke test checklist (Windows and macOS), rshot-ffmpeg bundled sidecar (+14 more)

### Community 22 - "Visual Direction Mockups"
Cohesion: 0.09
Nodes (23): countdown/index.html, editor/index.html, Mockup 01 Visual Direction (Glass, thumbnail 01-A), Direction B: Adwaita GNOME-native, Direction A: Glass macOS-like (chosen), Direction C: Signal developer tool (monospace, lime), Timer countdown ring, Mockup 03 Editor (+15 more)

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
Nodes (21): Record shortcut takeover (show-screen-recording-ui -> rshot record), rshot Plan 4: Windows/macOS + Installers, Platform differences behind #[cfg(target_os)] in owning modules, WH_KEYBOARD_LL hook with 0xE8 dummy key, NSIS installer hooks (hooks.nsh), GitHub Actions release workflow (tauri-action), macOS simple-fullscreen overlays, macOS symbolic hotkeys + global-shortcut plugin (+13 more)

### Community 27 - "Plan 1-2 Frontend Architecture"
Cohesion: 0.15
Nodes (20): crate::err (IPC errors as String), Glass style tokens (style A, glass.css), src/shared/ipc.ts typed invoke wrappers, overlay::Target (area | window | screen), selection.ts geometry (fromPoints, clamp, move, resize, handleAt, windowAt), Vite multi-page build (src/<page> -> dist/<page>), Crop with presets Free/16:9/4:3/1:1 (cropFromDrag, fitRatio), Editor interaction main.ts (tools, selection, text, crop, zoom, save/close) (+12 more)

### Community 28 - "Shortcut Commands Core"
Cohesion: 0.23
Nodes (16): command_for(), exe_command(), live_path(), manual_commands(), outdated(), release(), restore(), restore_and_save() (+8 more)

### Community 29 - "Small Window Frontends"
Cohesion: 0.14
Nodes (13): timer, next(), permissionStep(), rows, $(), bars, discard, mic (+5 more)

### Community 30 - "Shortcut Combo Parser"
Cohesion: 0.20
Nodes (15): Neutral shortcut combo parser (combo.rs), Per-OS default shortcuts table, bindable(), c(), Combo, Key, parse(), parse_key() (+7 more)

### Community 31 - "Editor Backend Commands"
Cohesion: 0.33
Nodes (17): Request, copy_path(), editor_delete(), editor_info(), editor_paths(), EditorInfo, open_editor(), save_image() (+9 more)

### Community 32 - "Video Trim UI"
Cohesion: 0.18
Nodes (14): usKey(), $(), changed(), copy(), done(), probe, render(), requestClose() (+6 more)

### Community 33 - "Plan 1 Foundation"
Cohesion: 0.17
Nodes (16): rshot Plan 1: Foundation + Linux Screenshots, Autonomous run environment notes (no sudo, xdo.py), cli::Cmd / cli::parse, Debian package (.deb), self-sufficient, crate::dispatch(&AppHandle, Cmd), ui::force_focus (X server timestamp present), Plan-1 manual checklist, overlay::start / overlay::Session (+8 more)

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
Cohesion: 0.24
Nodes (13): AppState (config, clipboard, last_capture, session, pending, thumb), Capture naming Screenshot_YYYY-MM-DD_HH-MM-SS.png (unique_path, new_screenshot_path), Floating thumbnail (thumbnail.rs, Thumb), ui::monitor_at, thumbnail::guard path guard, thumbnail::tildify, AppState.editors (label -> path map), editor.rs commands (open_editor, editor_info, save_image, editor_delete, copy_path) (+5 more)

### Community 38 - "Capture & Clipboard Plan"
Cohesion: 0.22
Nodes (13): store::write_atomic (temp file + rename), capture.rs (grab_all, frame_at, clamp_rect, crop, encode_png, windows_on, window_image), CI workflow Linux/Windows/macOS (ci.yml), clipboard::Clipboard owner thread (copy_capture, copy_image), Clipboard payload: path + image vs path only, clipboard-rs 0.3 crate, pipeline::finish_capture, Linux-only cfg gating, compile on Windows/macOS (+5 more)

### Community 39 - "macOS Bundle Config"
Cohesion: 0.17
Nodes (11): bundle, externalBin, macOS, resources, targets, hardenedRuntime, minimumSystemVersion, signingIdentity (+3 more)

### Community 40 - "TypeScript Config"
Cohesion: 0.17
Nodes (11): compilerOptions, isolatedModules, lib, module, moduleResolution, noEmit, skipLibCheck, strict (+3 more)

### Community 41 - "Settings & Onboarding Plan"
Cohesion: 0.27
Nodes (11): Mockup 05 Settings/Onboarding/Tray, GNOME shortcut takeover/restore (shortcuts::take_over, restore), shortcuts::gvariant (parse_strv, format_strv, to_gnome_accel, with_paths, without_paths), keys.ts comboFrom, Onboarding (Use rshot choice), Overlay options popover (options.ts, set_overlay_options, pick_folder), ui::popup / close_prefix / POPUP_SEQ, Settings window (settings.rs get_settings/set_settings) (+3 more)

### Community 42 - "Shortcut Recorder Keys"
Cohesion: 0.25
Nodes (6): comboFrom(), KeyLike, Mods, esc(), render(), update()

### Community 43 - "Thumbnail & Trim Helpers"
Cohesion: 0.31
Nodes (7): run(), setCopied(), clampTrim(), clock(), fmt(), playable(), timeAt()

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

## Knowledge Gaps
- **300 isolated node(s):** `name`, `version`, `doc`, `test`, `dev` (+295 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **3 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Autonomous run decisions log` connect `Design Spec & Perf Targets` to `Plan 1 Foundation`, `Cross-Check Script`, `Recording Flow Design`, `Bundled ffmpeg & Release`, `Visual Direction Mockups`, `Cross-Platform Plan 4`, `Shortcut Combo Parser`?**
  _High betweenness centrality (0.252) - this node is a cross-community bridge._
- **Why does `Trust the observed value on first takeover (+ macOS defaults-domain backup copy)` connect `Design Spec & Perf Targets` to `Cross-Platform Plan 4`, `macOS Shortcut Takeover`, `Windows Keyboard Hook`?**
  _High betweenness centrality (0.133) - this node is a cross-community bridge._
- **Why does `$()` connect `Small Window Frontends` to `Frontend IPC Wrappers`, `Visual Direction Mockups`, `Recording Flow Design`?**
  _High betweenness centrality (0.124) - this node is a cross-community bridge._
- **What connects `name`, `version`, `doc` to the rest of the system?**
  _303 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Recorder Backends` be split into smaller, more focused modules?**
  _Cohesion score 0.07434616514839847 - nodes in this community are weakly interconnected._
- **Should `Overlay Windows & Focus` be split into smaller, more focused modules?**
  _Cohesion score 0.08998435054773082 - nodes in this community are weakly interconnected._
- **Should `Config Store` be split into smaller, more focused modules?**
  _Cohesion score 0.09899749373433583 - nodes in this community are weakly interconnected._