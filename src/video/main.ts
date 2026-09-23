import '../shared/base';
import '../shared/glass.css';
import '../shared/chrome.css';
import './video.css';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ask, fail } from '../shared/ask';
import { mountIcons } from '../shared/icons';
import { usKey } from '../editor/model';
import * as ipc from '../shared/ipc';
import { clampTrim, clock, fmt, playable, timeAt } from './trim';

mountIcons();
const $ = <T extends HTMLElement = HTMLElement>(s: string) => document.querySelector(s) as T;
const v = $<HTMLVideoElement>('#v');
const strip = $('#strip');
const MIN_LEN = 0.5;

let ready = false; // until the video has loaded (or failed to), a close request just closes
// Registered before the awaits below, so the window can always be closed.
void getCurrentWindow().onCloseRequested(async (e) => {
  e.preventDefault();
  if (!ready) await ipc.closeWindow();
  else if ($('#modal').hidden) await requestClose(); // else a prompt is up: it gets answered first
});

const info = await ipc.editorInfo().catch(async (e) => {
  await fail("Couldn't open the recording", e);
  await ipc.closeWindow();
  throw e;
});
$('#title').textContent = info.name;
$('#path').textContent = info.display;
// Streamed from the loopback server (WebKitGTK's player can't read a custom URI scheme).
if (info.stream) v.src = info.stream;
/** The duration is known: the trim range means something. */
const loaded = !!info.stream && (await playable(v));
/** It can play and be edited here; false after a playback error. */
let ok = loaded;
const dur = loaded ? v.duration : 0;
let [start, end] = [0, dur];
let mute = false;
$('#meta').textContent = loaded ? `${clock(dur)} · ${v.videoWidth} × ${v.videoHeight} · MP4` : 'MP4';

/**
 * Can't play here (WebKitGTK needs gstreamer1.0-libav for H.264): the player goes, and Copy, Reveal
 * and Delete still work. A range chosen before a later playback error is kept: Done still trims it.
 */
function unplayable() {
  ok = false;
  v.pause();
  document.body.classList.add('noplay');
  if (!loaded) $('#readout').textContent = '';
}
if (!ok) unplayable();
v.addEventListener('error', unplayable);

const changed = () => loaded && (start > 0.05 || end < dur - 0.05 || mute);

function render() {
  if (!ok) return;
  const w = strip.clientWidth;
  const x = (t: number) => (t / dur) * w;
  $('#out-l').style.width = `${x(start)}px`;
  $('#out-r').style.width = `${w - x(end)}px`;
  $('#trim').style.left = `${x(start)}px`;
  $('#trim').style.width = `${x(end) - x(start)}px`;
  $('#head').style.left = `${x(v.currentTime)}px`;
  $('#readout').innerHTML = `<b>${fmt(v.currentTime)}</b> / ${fmt(dur)} &nbsp;·&nbsp; keeping <b>${fmt(start)} → ${fmt(end)}</b> (${(end - start).toFixed(1)} s)`;
  $('#mute').classList.toggle('on', mute);
  $('#mute').setAttribute('aria-checked', String(mute));
  $('#play').classList.toggle('playing', !v.paused);
  $('#bigplay').classList.toggle('playing', !v.paused);
}

/** Filmstrip: 12 evenly spaced frames, each filling its slot, seeked on a second, hidden video. */
const probe = Object.assign(document.createElement('video'), { muted: true, preload: 'auto' });
let probeReady: Promise<unknown> | undefined;
let strips = 0;
async function filmstrip() {
  const run = ++strips; // a newer run (after a resize) takes over
  const c = $<HTMLCanvasElement>('#frames');
  c.width = Math.round(strip.clientWidth * devicePixelRatio);
  c.height = Math.round(52 * devicePixelRatio);
  probeReady ??= new Promise((done) => {
    probe.addEventListener('loadeddata', done, { once: true });
    probe.src = v.src;
  });
  await probeReady;
  const g = c.getContext('2d')!;
  const n = 12;
  const [fw, h] = [c.width / n, c.height];
  const [vw, vh] = [probe.videoWidth, probe.videoHeight];
  const sw = Math.min(vw, (vh * fw) / h);
  const sh = (sw * h) / fw;
  for (let i = 0; i < n && run === strips; i++) {
    probe.currentTime = Math.max(0, Math.min(dur - 0.05, ((i + 0.5) / n) * dur));
    await new Promise((done) => probe.addEventListener('seeked', done, { once: true }));
    if (run === strips) g.drawImage(probe, (vw - sw) / 2, (vh - sh) / 2, sw, sh, i * fw, 0, fw, h);
  }
}

function toggle() {
  if (!ok) return;
  if (v.paused) {
    if (v.currentTime < start || v.currentTime >= end - 0.01) v.currentTime = start;
    void v.play();
  } else v.pause();
}

// While playing: the playhead follows every frame, and playback stops at the end of the kept range.
function tick() {
  if (v.paused) return;
  if (v.currentTime >= end) {
    v.pause();
    v.currentTime = start;
  }
  render();
  requestAnimationFrame(tick);
}
v.addEventListener('play', () => {
  render();
  requestAnimationFrame(tick);
});
v.addEventListener('pause', render);
v.addEventListener('seeked', render);
$('#play').addEventListener('click', toggle);
$('#bigplay').addEventListener('click', toggle);
v.addEventListener('click', toggle);
$('#mute').addEventListener('click', () => {
  mute = !mute;
  v.muted = mute;
  render();
});

// Dragging a grip moves that end; dragging elsewhere on the strip scrubs.
strip.addEventListener('pointerdown', (e) => {
  if (e.button !== 0 || !ok) return;
  const grip = (e.target as HTMLElement).dataset.grip as 'start' | 'end' | undefined;
  strip.setPointerCapture(e.pointerId);
  const apply = (ev: PointerEvent) => {
    const t = timeAt(ev.clientX - strip.getBoundingClientRect().left, strip.clientWidth, dur);
    if (grip === 'start') [start, end] = clampTrim(Math.min(t, end - MIN_LEN), end, dur, MIN_LEN);
    else if (grip === 'end') [start, end] = clampTrim(start, Math.max(t, start + MIN_LEN), dur, MIN_LEN);
    v.currentTime = grip === 'end' ? end : grip === 'start' ? start : t;
    render();
  };
  apply(e);
  const up = () => {
    strip.removeEventListener('pointermove', apply);
    strip.removeEventListener('pointerup', up);
  };
  strip.addEventListener('pointermove', apply);
  strip.addEventListener('pointerup', up);
});

async function copy(): Promise<boolean> {
  return ipc.retryCopy(info.path).then(
    () => true,
    (e) => fail("Couldn't copy", e).then(() => false),
  );
}

/** True once the file is trimmed (copied or not): it must never be trimmed twice with one range. */
async function trim(): Promise<boolean> {
  document.body.classList.add('saving');
  let copied = await ipc.trimVideo(start, end, mute).catch(async (e) => {
    document.body.classList.remove('saving'); // the prompt must stay clickable
    await fail("Couldn't save", e);
    return null;
  });
  document.body.classList.remove('saving');
  if (copied === null) return false;
  // The file is trimmed either way: a clipboard failure only offers a retry (spec §4).
  while (!copied && (await ask("Saved, but couldn't copy to the clipboard.", [{ label: 'OK', value: 'ok' }, { label: 'Retry', value: 'retry', primary: true }])) === 'retry') {
    copied = await ipc.retryCopy(info.path).then(() => true, () => false);
  }
  return true;
}

let busy = false;
async function done() {
  if (busy) return;
  busy = true;
  v.pause();
  if (await (changed() ? trim() : copy())) await ipc.closeWindow();
  busy = false;
}

async function requestClose() {
  if (busy) return;
  if (!changed()) return ipc.closeWindow();
  const r = await ask(`Save the trimmed ${info.name}?`, [
    { label: 'Discard', value: 'discard', danger: true },
    { label: 'Cancel', value: 'cancel' },
    { label: 'Save', value: 'save', primary: true },
  ]);
  if (r === 'discard') await ipc.closeWindow();
  else if (r === 'save') await done();
}

async function remove() {
  if (busy) return;
  const r = await ask(`Delete ${info.name}? This can't be undone.`, [
    { label: 'Cancel', value: 'cancel' },
    { label: 'Delete', value: 'delete', primary: true, danger: true },
  ]);
  if (r === 'delete') await ipc.editorDelete().catch((e) => fail("Couldn't delete", e));
}

$('#done').addEventListener('click', () => void done());
$('#copy').addEventListener('click', () => void copy());
$('#close').addEventListener('click', () => void requestClose());
$('#reveal').addEventListener('click', () => void ipc.revealCapture(info.path));
$('#delete').addEventListener('click', () => void remove());
// A mouse click neither focuses a button nor leaves focus on the last control, so Enter and Space
// stay the page's (as in the overlay); a Tab-focused button still takes them. Inputs keep focus.
addEventListener(
  'mousedown',
  (e) => {
    if (!(e.target as Element).closest('button')) return;
    e.preventDefault();
    (document.activeElement as HTMLElement | null)?.blur();
  },
  true,
);
addEventListener('keydown', (e) => {
  if (!$('#modal').hidden) return;
  // A focused control (a button clicked last, the mute switch) keeps Enter and Space for itself.
  if ((e.target as Element).closest('button, select, input') && (e.key === 'Enter' || e.key === ' ')) return;
  // Ctrl+C = Copy, on any layout (usKey maps the physical key, and ignores AltGr).
  if ((e.ctrlKey || e.metaKey) && [e.key.toLowerCase(), usKey(e)].includes('c')) {
    e.preventDefault();
    void copy();
  } else if (e.key === ' ') {
    e.preventDefault();
    toggle();
  } else if (e.key === 'Enter') {
    e.preventDefault();
    void done();
  } else if (e.key === 'Escape') void requestClose();
});
addEventListener('resize', () => {
  render();
  if (ok) void filmstrip();
});

render();
if (ok) void filmstrip();
ready = true;
