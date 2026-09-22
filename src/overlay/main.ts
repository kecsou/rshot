import '../shared/glass.css';
import './overlay.css';
import { emit, listen } from '@tauri-apps/api/event';
import { mountIcons } from '../shared/icons';
import * as ipc from '../shared/ipc';
import { closeOptions, optionsOpen, renderOptions, toggleOptions } from './options';
import { type Handle, type Rect, clamp, fromPoints, handleAt, move, resize, windowAt } from './selection';

mountIcons();

const q = <T extends HTMLElement = HTMLElement>(s: string) => document.querySelector(s) as T;
const canvas = q<HTMLCanvasElement>('#frame');
const selEl = q('#sel');
const dimEl = q('#dim');
const winhl = q('#winhl');
const winlabel = q('#winlabel');
const loupe = q('#loupe');
const loupeCtx = q<HTMLCanvasElement>('#loupe canvas').getContext('2d')!;
const coord = q('#coord');
const hint = q('#hint');
const bar = q('#bar');
const shade = q('#shade');

type Mode = ipc.OverlayInfo['mode'];
type Drag = { kind: 'new' | 'move' | Handle; ax: number; ay: number; start: Rect | null };

let info: ipc.OverlayInfo | null = null;
let pixels: Uint8ClampedArray<ArrayBuffer> | null = null;
let mode: Mode = 'area';
let sel: Rect | null = null; // image pixels
let drag: Drag | null = null;
let pointer: [number, number] | null = null; // image pixels
let busy = false;
let hides = 0; // bumped on overlay:hide so an in-flight load() of that session drops its frame

// Keep: non-active overlays wait for overlay:primary-ready (Task 6 perf gate): the overlay under the
// pointer gets the IPC bandwidth first, the others fetch their frames once it has painted (or after 500 ms).
let primaryReady = 0;
let latest = 0;
let wakeWaiter = () => {};
void listen<number>('overlay:primary-ready', (e) => {
  primaryReady = e.payload;
  wakeWaiter();
});
function afterPrimary(token: number): Promise<void> {
  if (primaryReady === token) return Promise.resolve();
  return new Promise((resolve) => {
    wakeWaiter = () => primaryReady === token && resolve();
    setTimeout(resolve, 500);
  });
}

/** Image pixels per CSS pixel (monitor scale); read live because the window size settles after show. */
const k = () => (info ? info.width / innerWidth : 1);
const toImg = (e: MouseEvent): [number, number] => [e.clientX * k(), e.clientY * k()];
const place = (el: HTMLElement, r: Rect) => {
  const s = k();
  Object.assign(el.style, { left: `${r.x / s}px`, top: `${r.y / s}px`, width: `${r.w / s}px`, height: `${r.h / s}px` });
};

async function load() {
  const gen = hides;
  const next = await ipc.overlayInfo();
  if (!next) return;
  latest = Math.max(latest, next.token);
  if (!next.active) await afterPrimary(next.token);
  const buf = await ipc.overlayFrame();
  if (next.token < latest || hides !== gen) return; // a newer session started, or this one ended, while it waited
  info = next;
  pixels = new Uint8ClampedArray(buf);
  canvas.width = next.width;
  canvas.height = next.height;
  canvas.getContext('2d')!.putImageData(new ImageData(pixels, next.width, next.height), 0, 0);
  mode = next.mode;
  sel = next.selection;
  drag = null;
  pointer = null;
  busy = false;
  renderOptions(next.options);
  render();
  await ipc.overlayReady(next.token);
}

function highlight(): { rect: Rect; title: string; id?: number } | null {
  if (!info) return null;
  if (mode === 'screen') return { rect: { x: 0, y: 0, w: info.width, h: info.height }, title: 'Screen' };
  if (mode !== 'window' || !pointer) return null;
  const w = windowAt(info.windows, pointer[0], pointer[1]);
  return w ? { rect: clamp(w, info.width, info.height), title: w.title || w.app, id: w.id } : null;
}

function render() {
  if (!info) return;
  document.body.dataset.mode = mode;
  bar.hidden = !info.active;
  hint.hidden = !(info.active && info.hints);
  bar.querySelectorAll<HTMLElement>('[data-mode]').forEach((b) => b.classList.toggle('on', b.dataset.mode === mode));
  shade.hidden = mode !== 'area' || !!sel;
  selEl.hidden = mode !== 'area' || !sel;
  if (sel) {
    place(selEl, sel);
    dimEl.textContent = `${Math.round(sel.w)} × ${Math.round(sel.h)}`;
  }
  const t = info.active || mode === 'window' ? highlight() : null;
  winhl.hidden = winlabel.hidden = !t;
  if (t) {
    place(winhl, t.rect);
    const s = k();
    Object.assign(winlabel.style, { left: `${(t.rect.x + t.rect.w / 2) / s}px`, top: `${(t.rect.y + t.rect.h / 2) / s}px` });
    winlabel.querySelector('b')!.textContent = t.title;
    winlabel.querySelector('small')!.textContent = `${Math.round(t.rect.w)} × ${Math.round(t.rect.h)} · click to capture`;
  }
  renderLoupe();
}

function renderLoupe() {
  const show = !!info && !!pixels && !!pointer && info.active && mode === 'area' && drag?.kind !== 'move' && !optionsOpen();
  loupe.hidden = coord.hidden = !show;
  if (!show || !info || !pixels || !pointer) return;
  const x = Math.min(info.width - 1, Math.max(0, Math.floor(pointer[0])));
  const y = Math.min(info.height - 1, Math.max(0, Math.floor(pointer[1])));
  loupeCtx.imageSmoothingEnabled = false;
  loupeCtx.fillStyle = '#000';
  loupeCtx.fillRect(0, 0, 110, 110);
  loupeCtx.drawImage(canvas, x - 5, y - 5, 11, 11, 0, 0, 110, 110);
  const i = (y * info.width + x) * 4;
  const hex = '#' + [pixels[i], pixels[i + 1], pixels[i + 2]].map((v) => v.toString(16).padStart(2, '0')).join('').toUpperCase();
  coord.innerHTML = `${x}, ${y}<i style="background:${hex}"></i>${hex}`;
  const s = k();
  const cx = pointer[0] / s;
  const cy = pointer[1] / s;
  const lx = cx + 18 + 114 > innerWidth ? cx - 18 - 114 : cx + 18;
  const ly = cy + 18 + 114 + 30 > innerHeight ? cy - 18 - 114 - 30 : cy + 18;
  Object.assign(loupe.style, { left: `${lx}px`, top: `${ly}px` });
  Object.assign(coord.style, { left: `${lx}px`, top: `${ly + 120}px` });
}

function setMode(m: Mode) {
  mode = m;
  closeOptions();
  render();
  void emit('overlay:mode', m); // every monitor follows the mode
}

function captureNow() {
  if (!info) return;
  if (mode === 'screen') void capture({ kind: 'screen' });
  else if (mode === 'window') {
    const t = highlight();
    if (t?.id !== undefined) void capture({ kind: 'window', id: t.id, rect: t.rect });
  } else if (sel && sel.w >= 1 && sel.h >= 1) void capture({ kind: 'area', rect: sel });
}

async function capture(target: ipc.Target) {
  if (!info || busy) return;
  busy = true;
  // Rust hides the overlays, reports failures as a notification, and resets on the next show.
  await ipc.overlayCapture(info.token, target).catch(() => {
    busy = false; // e.g. an IPC-level rejection: keep Esc working
  });
}

addEventListener('mousedown', (e) => {
  if (!info || busy || e.button !== 0 || (e.target as Element).closest('#bar, #pop')) return;
  closeOptions();
  if (!info.active) {
    info.active = true;
    void ipc.overlayActivate(info.token);
  }
  pointer = toImg(e);
  const [x, y] = pointer;
  if (mode !== 'area') return captureNow();
  const h = sel ? handleAt(sel, x, y, 8 * k()) : null;
  drag =
    h === 'inside'
      ? { kind: 'move', ax: x, ay: y, start: sel }
      : h
        ? { kind: h, ax: x, ay: y, start: sel }
        : { kind: 'new', ax: x, ay: y, start: null };
  if (drag.kind === 'new') sel = null;
  render();
});

addEventListener('mousemove', (e) => {
  if (!info) return;
  pointer = toImg(e);
  const [x, y] = pointer;
  const W = info.width;
  const H = info.height;
  if (drag?.kind === 'new') sel = clamp(fromPoints(drag.ax, drag.ay, x, y, e.shiftKey), W, H);
  else if (drag?.kind === 'move' && drag.start) sel = move(drag.start, x - drag.ax, y - drag.ay, W, H);
  else if (drag?.start) sel = clamp(resize(drag.start, drag.kind as Handle, x, y), W, H);
  render();
});

addEventListener('mouseup', () => {
  if (drag?.kind === 'new' && sel && (sel.w < 3 || sel.h < 3)) sel = null;
  drag = null;
  render();
});

addEventListener('dblclick', (e) => {
  if (mode === 'area' && sel && handleAt(sel, ...toImg(e), 0) === 'inside') captureNow();
});

addEventListener('keydown', (e) => {
  if (!info || busy) return;
  if (e.key === 'Escape') {
    if (optionsOpen()) closeOptions();
    else void ipc.overlayCancel();
  } else if (e.key === 'Enter') captureNow();
  else if (e.key === ' ') {
    e.preventDefault();
    setMode(mode === 'window' ? 'area' : 'window');
  } else if (e.key.startsWith('Arrow') && sel && mode === 'area') {
    e.preventDefault();
    const d = e.shiftKey ? 10 : 1;
    const step: Record<string, [number, number]> = { ArrowLeft: [-d, 0], ArrowRight: [d, 0], ArrowUp: [0, -d], ArrowDown: [0, d] };
    const [dx, dy] = step[e.key] ?? [0, 0];
    sel = move(sel, dx, dy, info.width, info.height);
    render();
  }
});

bar.addEventListener('click', (e) => {
  const b = (e.target as Element).closest<HTMLElement>('button');
  if (!b) return;
  if (b.dataset.mode) setMode(b.dataset.mode as Mode);
  else if (b.dataset.act === 'cancel') void ipc.overlayCancel();
  else if (b.dataset.act === 'capture') captureNow();
  else if (b.dataset.act === 'options') toggleOptions();
});

void listen<number>('overlay:show', () => void load().catch(() => {}));
void listen<Mode>('overlay:mode', (e) => {
  mode = e.payload;
  if (mode !== 'area') drag = null;
  render();
});
void listen<number>('overlay:active', (e) => {
  if (!info) return;
  info.active = e.payload === info.index;
  if (!info.active) sel = null;
  render();
});
void listen('overlay:hide', () => {
  hides++;
  info = null;
  pixels = null;
  sel = null;
  drag = null;
  canvas.width = canvas.height = 0;
  closeOptions();
});
void load().catch(() => {}); // e.g. overlay_frame's "no frame" once the session has ended
