# Performance gate (spec §5) — 2026-09-22

Machine: Ubuntu 24.04, GNOME 46, X11, NVIDIA GTX 1080 (driver/library mismatch pending a reboot, so EGL
falls back to software); monitors DP-1 1920×1080 @ +0+224, DP-4 2560×1600 @ +1920+0, HDMI-0 1920×1080 @ +4480+252.
Build: release (`npm run tauri build -- --no-bundle`), preloaded overlays. rshot now sets
`WEBKIT_DISABLE_DMABUF_RENDERER=1` itself (WebKitGTK aborted with `EGL_NOT_INITIALIZED` without it); every run
below started the daemon **without** that variable in the environment.

## Method

- Trigger: temporary GNOME custom keybinding `<Ctrl><Super>F12` → `…/target/release/rshot capture area`
  (same path as `Print`), pressed with `scripts/xdo.py` (XTest, so GNOME's grab sees a real key press).
- key → visible: T0 is taken in the driver right before the XTest press; "visible" is the epoch in the daemon's
  `overlay visible at` line (logged after `show()` + `set_focus()` of the overlay under the pointer).
- Breakdown per run: launch = `overlay requested at` − T0 (GNOME spawns a second rshot, which forwards argv over
  single-instance and exits); grab = `grabbed … in X ms`; place/IPC/paint = the rest.
- Focus: `xdo.py active` 1 s after the press, then `key Escape 0.12` (held 120 ms) and count viewable
  `rshot overlay` windows. A key is only sent when an overlay is focused (a stray Esc/Enter must not hit the
  terminal); on a focus failure the stuck overlays are cleared by restarting the daemon.
- Right-overlay check: pointer warped to each monitor's centre (`xdo.py move`), binding pressed, 0.3 s reaction
  time, `key Return 0.12` (captures the focused overlay's whole monitor), then the new PNG's size must be that
  monitor's size. First key after Esc: open, Esc, reopen, Esc must hide it. Test PNGs were deleted afterwards.

## A. Preloaded overlays, as built by Task 5 (no fix)

| Run | key → visible (ms) | in-process (ms) | grab (ms) | focus OK | hidden after Esc |
|---|---|---|---|---|---|
| 1 | 341 | 257 | 67 | yes | yes (0 left) |
| 2 | 291 | 206 | 58 | **no** (terminal kept focus) | no (3 left) |
| 3 | 339 | 255 | 64 | yes (fresh daemon) | yes |
| 4 | 326 | 245 | 61 | **no** | no |
| 5 | 361 | 276 | 72 | yes (fresh daemon) | yes |

Right overlay via Enter: DP-1 243 ms → 1920×1080 OK; DP-4 333 ms → **focus on terminal, no capture**;
HDMI-0 298 ms → 1920×1080 OK. First key after Esc: open 1 focused + hidden; open 2 **not focused**.
(An earlier aborted attempt showed the same: run 1 329 ms OK, run 2 295 ms focus lost, 3 overlays stuck.)

Pattern: an overlay gets focus the first time it maps, and is refused focus every time it is re-shown after
having held focus once. Mutter's focus-stealing prevention compares the window's stale `_NET_WM_USER_TIME`
(the previous Esc) with the newer focus time of the window that took over. **Focus check failed.**

## B. Preloaded overlays + `force_focus` (brief fallback: fresh X server time + `present_with_time`)

| Run | key → visible (ms) | launch | grab | place/IPC/paint | focus OK | hidden after Esc |
|---|---|---|---|---|---|---|
| 1 | 353 | – | 65 | – | yes | yes |
| 2 | 316 | – | 57 | – | yes | yes |
| 3 | 297 | – | 60 | – | yes | yes |
| 4 | 293 | – | 64 | – | yes | yes |
| 5 | 290 | – | 73 | – | yes | yes |
| 6 | 308 | 82 | 64 | 162 | yes | yes |
| 7 | 307 | 81 | 63 | 163 | yes | yes |
| 8 | 344 | 99 | 71 | 174 | yes | yes |
| 9 | 337 | 86 | 72 | 179 | yes | yes |
| 10 | 301 | 92 | 68 | 141 | yes | yes |

Final run on the committed code (clean rebuild):

| Run | key → visible (ms) | launch | grab | place/IPC/paint | focus OK | hidden after Esc |
|---|---|---|---|---|---|---|
| 1 | 350 | 80 | 64 | 206 | yes | yes |
| 2 | 320 | 87 | 63 | 170 | yes | yes |
| 3 | 279 | 83 | 65 | 131 | yes | yes |
| 4 | 304 | 84 | 66 | 154 | yes | yes |
| 5 | 313 | 81 | 67 | 165 | yes | yes |

Right overlay via Enter (both B runs): DP-1 309 / 256 ms → 1920×1080 OK; DP-4 327 / 301 ms → 2560×1600 OK;
HDMI-0 292 / 302 ms → 1920×1080 OK. First key after Esc: 308 / 328 ms and 300 / 316 ms, both hidden (0 left).
**Focus: pass.** **Latency: fail** — 279–353 ms against ≤ 250 ms, on every run.

## Memory (daemon + WebKit children)

| State | RSS sum (brief's metric) | PSS sum | Private dirty |
|---|---|---|---|
| Preloaded, fresh idle | 734 / 743 MB | 247 / 250 MB | 116 / 129 MB |
| Preloaded, idle after ~15 overlays | 915 / 910 MB | 392 / 393 MB | 243 / 244 MB |

Five processes: rshot, WebKitNetworkProcess and one WebKitWebProcess per overlay (≈ 170–190 MB RSS each, of
which ≈ 140 MB is shared libwebkit pages; ≈ 55–75 MB PSS each). After use the rshot process itself grows from
≈ 30 MB to ≈ 118 MB anonymous memory: freed frame buffers stay in malloc's arenas. **Memory: fail** on both RSS
and PSS.

## C. Experiment, not kept: on-demand overlays (brief's memory fallback)

`ensure_overlays` removed from `setup`, `destroy()` instead of `hide()`:

| Run | key → visible (ms) | launch | grab | place/IPC/paint | focus OK |
|---|---|---|---|---|---|
| 1 | 548 | 83 | 65 | 400 | yes |
| 2 | 496 | 84 | 64 | 348 | **no** |
| 3 | 502 | 86 | 64 | 352 | yes |
| 4 | 481 | 87 | 78 | 316 | **no** |
| 5 | 575 | 87 | 66 | 422 | yes |

Idle: 54 MB RSS / 35 MB PSS fresh, 292 MB RSS / 181 MB PSS after five overlays (rshot alone 249 MB RSS,
the WebKit web processes exit with their windows). Latency roughly doubles, so it was reverted.

## Decision, round 1

Applied `force_focus`: focus now lands on the overlay under the pointer on every run and Esc/Enter work on
the first press. **BLOCKED on latency** (and memory): key → visible is 279–353 ms, over the 250 ms target.

Breakdown of the ≈ 310 ms median: launch ≈ 85 ms (GNOME spawning a second rshot that initialises GTK/Tauri
only to forward argv), grab ≈ 65 ms (xcap, 3 monitors), place/IPC/paint ≈ 130–210 ms (three pages each fetch
`overlay_info` + an 8–16 MB raw frame over IPC, `putImageData`, then `overlay_ready`). Even with zero launch
cost the in-process part alone (195–276 ms) misses the target on some runs. Options for the controller:
cheaper frames to the page (PNG/downscaled preview), a native overlay, and/or removing the launch cost
(daemon-owned key grab or forwarding before Tauri starts); for memory, `malloc_trim` after a session and fewer
WebKit processes.

## Round 2: controller's optimisations (same method, same machine)

Changes: (1) a second `rshot capture …` hands argv to the daemon over the single-instance plugin's D-Bus
interface before GTK/Tauri start (falls through to becoming the daemon when none answers); (2) monitors grabbed
on scoped threads; (3) non-active overlays wait for `overlay:primary-ready` (or 500 ms) before fetching their
frames; (4) returning freed frames to the OS, see the memory section.

### D1. Items 1–4 as specified (`malloc_trim(0)` after each session)

| Run | key → visible (ms) | launch | grab | place/IPC/paint | focus OK | hidden after Esc |
|---|---|---|---|---|---|---|
| 1 | 227 | 42 | 45 | 140 | yes | yes |
| 2 | 219 | 40 | 46 | 133 | yes | yes |
| 3 | 220 | 44 | 53 | 123 | yes | yes |
| 4 | 231 | 46 | 36 | 149 | yes | yes |
| 5 | 199 | 43 | 36 | 120 | yes | yes |

Enter: DP-1 175 ms → 1920×1080 OK, DP-4 212 ms → 2560×1600 OK, HDMI-0 168 ms → 1920×1080 OK. First key after
Esc: 227 / 203 ms, both hidden (a rebuild of the same code gave 247 / 238 ms). **Latency passes (12/12, 168–247 ms).**

Memory: fresh idle 746 MB RSS / 250 MB PSS; 3 s after use **943 MB RSS / 415 MB PSS**: `malloc_trim` released
nothing. `malloc_stats()` right after the trim showed glibc arenas 1, 2, 5 and 10 each holding 16.4–16.6 MB of
system memory with 2–10 KB in use. glibc's dynamic mmap threshold rises to the size of the first freed frame,
so later frames are allocated in per-thread arenas. `malloc_trim` only trims the top of the main arena, so
those arenas keep their free 16 MB tops.

### D2. Fixed mmap threshold instead (`mallopt(M_MMAP_THRESHOLD, 1 MB)` at daemon start, committed)

Frames are now mmapped and unmapped on free. With this in place `malloc_trim` made no measurable difference
(rshot anonymous memory is 30–31 MB after use either way), so it was dropped. Final code, clean rebuild:

| Run | key → visible (ms) | launch | grab | place/IPC/paint | focus OK | hidden after Esc |
|---|---|---|---|---|---|---|
| 1 | 235 | 37 | 45 | 153 | yes | yes |
| 2 | 243 | 39 | 59 | 145 | yes | yes |
| 3 | 233 | 39 | 52 | 142 | yes | yes |
| 4 | 250 | 37 | 48 | 165 | yes | yes |
| 5 | **257** | 40 | 44 | 173 | yes | yes |
| 6 | 224 | 40 | 47 | 137 | yes | yes |
| 7 | 223 | 37 | 46 | 140 | yes | yes |
| 8 | 225 | 39 | 64 | 122 | yes | yes |
| 9 | 222 | 40 | 46 | 136 | yes | yes |
| 10 | 217 | 39 | 44 | 134 | yes | yes |

Enter: DP-1 193 ms → 1920×1080 OK, DP-4 226 ms → 2560×1600 OK, HDMI-0 164 ms → 1920×1080 OK. First key after
Esc: 224 / 211 ms, both hidden. The same allocator setting with `malloc_trim` kept (22 runs, 5 of them instrumented) gave 209–265 ms,
with 265 and 256 over the target. **Latency: 3 of 37 runs over 250 ms** (257, 256, 265; mean ≈ 229 ms), about
15 ms slower than D1 because every session now page-faults its frame buffers in again. D1 reuses memory that
is already resident. Run 1 after a daemon start is always cold, whichever build is used.

| Memory (daemon + WebKit) | RSS sum | PSS sum | Private dirty |
|---|---|---|---|
| Fresh idle | 742 MB | 247 MB | 126 MB |
| Idle, 10–40 s after use | 819–822 MB | **302–304 MB** | 151–153 MB |

After use, rshot is back to its fresh 30 MB anonymous memory (81 MB PSS). The remaining growth is in the
WebKit web processes: 35–43 MB anonymous each vs ≈ 26 MB fresh, which is JS/canvas memory the page no longer
references.

### Where the remaining time goes (temporary instrumentation, one D2 run, ms after trigger)

Grab done +56, overlays placed +57. The three `overlay_info` calls run one at a time on the main thread from +58
to +84 (the window listing takes 8–9 ms each). The active frame is fetched at +85 and the active overlay is
ready at +202: **≈ 117 ms to move 16 MB through IPC, `putImageData` and paint** (software rendering on this
machine). The other overlays are ready at +274 and +304. Making `overlay_info` async gave no measurable gain in
15 runs (205–257 ms, 2 over 250), so it was not kept.

Forwarding checks: with no daemon, `rshot capture area` became the daemon and showed a focused overlay (Esc hid
it). A second plain `rshot` exited 0 in 0.07 s through the plugin, as before.

## Decision, round 2

**BLOCKED, narrowly.** Launch is ≈ 40 ms (was 85) and grab ≈ 45 ms (was 65). Focus, Esc and Enter pass on
every run. With memory actually returned to the OS, key → visible is 164–257 ms on the committed build (3 of
37 runs over 250 ms across both builds that use the fixed threshold), and idle-after-use PSS is 302–304 MB.
Without returning memory (D1), latency passes (≤ 247 ms) but PSS after use is 415 MB. The remaining big item
is the ≈ 117 ms that it takes to move the 16 MB active frame into WebKit and paint it. Smaller frames to the
page would cut both that time and the page faults.
