import '../shared/base';
import '../shared/glass.css';
import './editor.css';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { mountIcons } from '../shared/icons';
import * as ipc from '../shared/ipc';
import * as M from './model';
import { drawCrop, drawDoc, drawSelection, exportPng, ink, measureText } from './render';
import { type Tool, SECTIONS, closePopover, currentColor, onStyleChange, openPopover, popoverOpen, style } from './styles';

mountIcons();

const $ = <T extends HTMLElement = HTMLElement>(s: string) => document.querySelector(s) as T;
const stage = $('#stage');
const canvas = $<HTMLCanvasElement>('#view');
const ctx = canvas.getContext('2d')!;
const texted = $<HTMLTextAreaElement>('#texted');

let ready = false; // until the capture is loaded, a close request just closes
// Registered before the awaits below, so the window can always be closed.
void getCurrentWindow().onCloseRequested(async (e) => {
  e.preventDefault();
  if (!ready) await ipc.closeWindow();
  else if ($('#modal').hidden) await requestClose(); // else a prompt is up: it gets answered first
});

// A prompt holds the keyboard: Tab cycles its buttons, Esc picks the safe one, Enter the focused one.
addEventListener(
  'keydown',
  (e) => {
    if ($('#modal').hidden) return;
    e.stopPropagation();
    const bs = [...$('#modal-acts').children] as HTMLElement[];
    if (e.key === 'Escape') {
      e.preventDefault();
      (bs.find((b) => b.classList.contains('b2') && !b.classList.contains('danger')) ?? bs[0]).click();
    } else if (e.key === 'Tab') {
      e.preventDefault();
      const i = bs.indexOf(document.activeElement as HTMLElement);
      bs[i < 0 ? 0 : (i + (e.shiftKey ? bs.length - 1 : 1)) % bs.length].focus();
    }
  },
  true,
);

/** Startup failed: say why, then close (nothing to edit). */
async function fatal(name: string, e: unknown): Promise<never> {
  await ask(`Couldn't open ${name}: ${e}`, [{ label: 'OK', value: 'ok', primary: true }]);
  await ipc.closeWindow();
  throw e;
}

const info = await ipc.editorInfo().catch((e) => fatal('the capture', e));
const img = await ipc
  .readCapture(info.path)
  .then((b) => createImageBitmap(new Blob([b], { type: 'image/png' })))
  // Text boxes are measured on the canvas: measuring with the fallback font would size them wrong.
  .then(async (bmp) => (await Promise.all(['600', '700'].map((w) => document.fonts.load(`${w} 16px Inter`))), bmp))
  .catch((e) => fatal(info.name, e));
const W = img.width;
const H = img.height;
const u = M.unitFor(W, H);
const hist = new M.History({ shapes: [], crop: null });
$('#title').textContent = info.name;
$('#path').textContent = info.display;

type Drag =
  | { kind: 'draw'; start: M.Pt; shape: M.Shape }
  | { kind: 'move'; start: M.Pt; orig: M.Shape }
  | { kind: 'handle'; handle: M.HandleId; orig: M.Shape }
  | { kind: 'crop-new'; start: M.Pt; prev: M.Rect }
  | { kind: 'crop-move'; start: M.Pt; orig: M.Rect }
  | { kind: 'crop-corner'; fixed: M.Pt };

let tool: Tool = 'arrow';
let selectedId: number | null = null;
let live: M.Doc | null = null; // preview while dragging; committed on pointerup
let drag: Drag | null = null;
let zoom = 1;
let fitMode = true;
let nextId = 1;
let editingText: { id: number | null; at: M.Pt } | null = null;
let crop: { draft: M.Rect; ratio: M.Ratio } | null = null;
let busy = false;

const doc = () => live ?? hist.doc;
const selected = () => doc().shapes.find((s) => s.id === selectedId) ?? null;
const full = (): M.Rect => ({ x: 0, y: 0, w: W, h: H });
/** What the canvas shows: the whole capture while cropping, otherwise the cropped area. */
const viewRect = (): M.Rect => (crop ? full() : (doc().crop ?? full()));

// ---------- painting ----------

function paint() {
  const v = viewRect();
  const cw = Math.round(v.w);
  const ch = Math.round(v.h);
  if (canvas.width !== cw || canvas.height !== ch) {
    canvas.width = cw;
    canvas.height = ch;
  }
  canvas.style.width = `${cw * zoom}px`;
  canvas.style.height = `${ch * zoom}px`;
  ctx.setTransform(1, 0, 0, 1, -v.x, -v.y);
  ctx.clearRect(v.x, v.y, v.w, v.h);
  drawDoc(ctx, img, doc(), u, editingText?.id ?? undefined);
  const s = selected();
  if (s && !crop && !editingText) drawSelection(ctx, s, zoom, u);
  if (crop) drawCrop(ctx, crop.draft, W, H, zoom);
}

function render() {
  if (fitMode) fit(); // the view changes size when a crop is applied, undone or redone
  paint();
  document.body.dataset.tool = tool;
  $('#undo').toggleAttribute('disabled', !hist.canUndo);
  $('#redo').toggleAttribute('disabled', !hist.canRedo);
  $('#dirty').hidden = !hist.dirty;
  $('#zoom-val').textContent = `${Math.round(zoom * 100)}%`;
  const out = hist.doc.crop ?? full();
  $('#meta').textContent = `${Math.round(out.w)} × ${Math.round(out.h)} · PNG`;
  document.querySelectorAll<HTMLElement>('#rail [data-tool]').forEach((b) => b.classList.toggle('on', b.dataset.tool === tool));
  $('#swatch i').style.background = currentColor(tool);
  if (crop) $('#cropdim').textContent = `${Math.round(crop.draft.w)} × ${Math.round(crop.draft.h)}`;
  document.querySelectorAll<HTMLElement>('#ratios [data-ratio]').forEach((b) => b.classList.toggle('on', b.dataset.ratio === crop?.ratio));
  placeTextarea(); // follows zoom and window resizes while editing
}

function commit(next: M.Doc) {
  hist.commit(next);
  render();
}

function replaceShape(next: M.Shape) {
  commit({ ...hist.doc, shapes: hist.doc.shapes.map((s) => (s.id === next.id ? next : s)) });
}

// ---------- zoom ----------

function fit() {
  fitMode = true;
  const v = viewRect();
  zoom = M.fitZoom(v.w, v.h, stage.clientWidth - 140, stage.clientHeight - (crop ? 110 : 60));
}

function setZoom(z: number) {
  fitMode = false;
  zoom = z;
  render();
}

addEventListener('resize', () => render());

stage.addEventListener(
  'wheel',
  (e) => {
    if (!e.ctrlKey) return;
    e.preventDefault();
    setZoom(M.zoomStep(zoom, e.deltaY < 0 ? 1 : -1));
  },
  { passive: false },
);

// ---------- styles ----------

function adoptStyle(s: M.Shape) {
  if (s.kind === 'highlight') style.highlight = s.color;
  else if ('color' in s) style.color = s.color;
  if ('width' in s) style.width = s.width;
  if (s.kind === 'arrow') {
    style.ends = s.ends;
    style.shadow = s.shadow;
  }
  if (s.kind === 'text') {
    style.size = s.size;
    style.textStyle = s.style;
  }
  if (s.kind === 'redact') {
    style.mode = s.mode;
    style.strength = s.strength;
  }
}

function restyle(s: M.Shape): M.Shape {
  switch (s.kind) {
    case 'arrow':
      return { ...s, color: style.color, width: style.width, ends: style.ends, shadow: style.shadow };
    case 'line':
    case 'rect':
    case 'ellipse':
    case 'pen':
      return { ...s, color: style.color, width: style.width };
    case 'highlight':
      return { ...s, color: style.highlight, width: style.width };
    case 'counter':
      return { ...s, color: style.color };
    case 'redact':
      return { ...s, mode: style.mode, strength: style.strength };
    case 'text':
      return { ...s, color: style.color, size: style.size, style: style.textStyle, ...measureText(ctx, s.text, M.TEXT_PX[style.size] * u, style.textStyle) };
  }
}

onStyleChange(() => {
  const s = hist.doc.shapes.find((x) => x.id === selectedId);
  if (s) replaceShape(restyle(s));
  render();
});

function select(s: M.Shape | null) {
  selectedId = s?.id ?? null;
  if (s) adoptStyle(s);
}

// ---------- tools ----------

function setTool(t: Tool) {
  if (crop && t !== 'crop') exitCrop(false);
  if (t === 'crop') {
    if (!crop) enterCrop();
    return;
  }
  if (t === tool && SECTIONS[t].length) {
    openPopover(t, $(`#rail [data-tool="${t}"]`));
    return;
  }
  tool = t;
  closePopover();
  render();
}

function newShape(t: Tool, p: M.Pt): M.Shape | null {
  const id = nextId++;
  switch (t) {
    case 'arrow':
      return { id, kind: 'arrow', a: p, b: p, color: style.color, width: style.width, ends: style.ends, shadow: style.shadow };
    case 'line':
    case 'rect':
    case 'ellipse':
      return { id, kind: t, a: p, b: p, color: style.color, width: style.width };
    case 'redact':
      return { id, kind: 'redact', a: p, b: p, mode: style.mode, strength: style.strength };
    case 'pen':
      return { id, kind: 'pen', pts: [p], color: style.color, width: style.width };
    case 'highlight':
      return { id, kind: 'highlight', pts: [p], color: style.highlight, width: style.width };
    default:
      return null;
  }
}

function toImage(e: MouseEvent): M.Pt {
  const r = canvas.getBoundingClientRect();
  return M.viewToImage({ x: e.clientX - r.left, y: e.clientY - r.top }, zoom, viewRect());
}

// ---------- pointer ----------

canvas.addEventListener('pointerdown', (e) => {
  if (e.button !== 0) return;
  closePopover();
  if (editingText) return commitText();
  canvas.setPointerCapture(e.pointerId);
  const p = toImage(e);
  if (crop) return startCropDrag(p);
  const sel = selected();
  const handle = sel && M.handles(sel).find((h) => Math.hypot(h.p.x - p.x, h.p.y - p.y) <= 9 / zoom);
  if (sel && handle) {
    drag = { kind: 'handle', handle: handle.id, orig: sel };
    return;
  }
  if (tool === 'select') {
    const hit = M.hitTest(hist.doc, p, 6 / zoom, u);
    select(hit);
    if (hit) drag = { kind: 'move', start: p, orig: hit };
    return render();
  }
  if (tool === 'text') {
    const hit = M.hitTest(hist.doc, p, 4 / zoom, u);
    return startText(hit?.kind === 'text' ? hit : null, p);
  }
  if (tool === 'counter') {
    const id = nextId++;
    commit({ ...hist.doc, shapes: [...hist.doc.shapes, { id, kind: 'counter', at: p, n: M.nextCounter(hist.doc), color: style.color }] });
    selectedId = id;
    return render();
  }
  const shape = newShape(tool, p);
  if (!shape) return;
  drag = { kind: 'draw', start: p, shape };
  live = { ...hist.doc, shapes: [...hist.doc.shapes, shape] };
  paint();
});

// pointerdown may have just opened the text box: the click mustn't blur (and so commit) it.
canvas.addEventListener('mousedown', (e) => {
  if (editingText) e.preventDefault();
});

canvas.addEventListener('pointermove', (e) => {
  if (!drag) return;
  const p = toImage(e);
  if (drag.kind === 'crop-new' || drag.kind === 'crop-move' || drag.kind === 'crop-corner') return moveCropDrag(p);
  const base = hist.doc;
  let next: M.Shape;
  if (drag.kind === 'draw') {
    const s = drag.shape;
    if ('pts' in s) {
      s.pts.push(p); // the draft isn't committed yet, so mutating it is safe
      next = s;
    } else if (s.kind === 'text' || s.kind === 'counter') return;
    else {
      const b = !e.shiftKey ? p : s.kind === 'arrow' || s.kind === 'line' ? M.snap45(drag.start, p) : M.squareEnd(drag.start, p);
      next = { ...s, b };
      drag.shape = next;
    }
    live = { ...base, shapes: [...base.shapes.filter((x) => x.id !== next.id), next] };
  } else {
    next = drag.kind === 'move' ? M.translate(drag.orig, p.x - drag.start.x, p.y - drag.start.y) : M.dragHandle(drag.orig, drag.handle, p, e.shiftKey);
    live = { ...base, shapes: base.shapes.map((x) => (x.id === next.id ? next : x)) };
  }
  paint();
});

canvas.addEventListener('pointerup', () => {
  const d = drag;
  drag = null;
  if (!d) return;
  if (d.kind === 'crop-new' || d.kind === 'crop-move' || d.kind === 'crop-corner') {
    if (d.kind === 'crop-new' && crop && (crop.draft.w < 3 || crop.draft.h < 3)) crop.draft = d.prev; // a click, not a drag
    return render();
  }
  const next = live;
  live = null;
  if (!next) return render();
  if (d.kind === 'draw') {
    const s = next.shapes[next.shapes.length - 1];
    const b = M.bounds(s, u);
    const tiny = s.kind === 'pen' || s.kind === 'highlight' ? s.pts.length < 2 : Math.max(b.w, b.h) < 3;
    if (tiny) return render();
    selectedId = s.id;
  }
  commit(next);
});

canvas.addEventListener('dblclick', (e) => {
  const hit = M.hitTest(hist.doc, toImage(e), 4 / zoom, u);
  if (hit?.kind === 'text') startText(hit, hit.at);
});

// ---------- text ----------

function placeTextarea() {
  if (!editingText) return;
  const v = viewRect();
  const px = M.TEXT_PX[style.size] * u * zoom;
  texted.dataset.style = style.textStyle;
  texted.style.setProperty('--c', style.color);
  texted.style.setProperty('--ink', ink(style.color));
  Object.assign(texted.style, {
    left: `${canvas.offsetLeft + (editingText.at.x - v.x) * zoom}px`,
    top: `${canvas.offsetTop + (editingText.at.y - v.y) * zoom}px`,
    fontSize: `${px}px`,
    fontFamily: 'Inter, system-ui, sans-serif',
  });
  autosize();
}

function autosize() {
  const m = measureText(ctx, texted.value || ' ', M.TEXT_PX[style.size] * u, style.textStyle);
  texted.style.width = `${m.w * zoom + 8}px`;
  texted.style.height = `${m.h * zoom + 4}px`;
}

function startText(existing: Extract<M.Shape, { kind: 'text' }> | null, p: M.Pt) {
  if (existing) adoptStyle(existing);
  editingText = { id: existing?.id ?? null, at: existing?.at ?? p };
  texted.value = existing?.text ?? '';
  texted.hidden = false;
  placeTextarea();
  texted.focus();
  render();
}

function commitText(cancel = false) {
  if (!editingText) return;
  const { id, at } = editingText;
  editingText = null;
  // Hiding alone leaves the box focused for the input method, which then types the next tool key into it.
  texted.blur();
  texted.hidden = true;
  const value = cancel && id === null ? '' : texted.value.replace(/\s+$/, '');
  const rest = hist.doc.shapes.filter((s) => s.id !== id);
  if (value) {
    const px = M.TEXT_PX[style.size] * u;
    const shape: M.Shape = { id: id ?? nextId++, kind: 'text', at, text: value, color: style.color, size: style.size, style: style.textStyle, ...measureText(ctx, value, px, style.textStyle) };
    const i = hist.doc.shapes.findIndex((s) => s.id === id); // keep z-order when re-editing
    const shapes = [...rest];
    shapes.splice(i >= 0 ? i : shapes.length, 0, shape);
    selectedId = shape.id;
    commit({ ...hist.doc, shapes });
  } else if (id !== null) commit({ ...hist.doc, shapes: rest });
  else render();
}

texted.addEventListener('input', autosize);
texted.addEventListener('keydown', (e) => {
  e.stopPropagation();
  if (e.isComposing) return; // Enter/Escape belong to the input method while it composes
  const k = [e.key.toLowerCase(), M.usKey(e)].find((x) => x === 's' || x === 'c');
  if ((e.ctrlKey || e.metaKey) && k) {
    e.preventDefault();
    void (k === 's' ? done() : copy()); // both commit the text first
  } else if (e.key === 'Enter' && !e.shiftKey) {
    e.preventDefault();
    commitText();
  } else if (e.key === 'Escape') {
    e.preventDefault();
    commitText(true);
  }
});
texted.addEventListener('blur', () => commitText());

// ---------- crop ----------

function enterCrop() {
  tool = 'crop';
  crop = { draft: hist.doc.crop ?? full(), ratio: 'free' };
  selectedId = null;
  closePopover();
  $('#cropbar').hidden = false;
  fit();
  render();
}

function exitCrop(apply: boolean) {
  if (!crop) return;
  const r = { x: Math.round(crop.draft.x), y: Math.round(crop.draft.y), w: Math.round(crop.draft.w), h: Math.round(crop.draft.h) };
  crop = null;
  tool = 'select';
  $('#cropbar').hidden = true;
  const cur = hist.doc.crop ?? full();
  const same = r.x === cur.x && r.y === cur.y && r.w === cur.w && r.h === cur.h;
  if (apply && !same && r.w >= 1 && r.h >= 1) {
    const whole = r.x === 0 && r.y === 0 && r.w === W && r.h === H;
    commit({ ...hist.doc, crop: whole ? null : r });
  }
  fit();
  render();
}

function startCropDrag(p: M.Pt) {
  if (!crop) return;
  const r = crop.draft;
  const corners: [M.Pt, M.Pt][] = [
    [{ x: r.x, y: r.y }, { x: r.x + r.w, y: r.y + r.h }],
    [{ x: r.x + r.w, y: r.y }, { x: r.x, y: r.y + r.h }],
    [{ x: r.x, y: r.y + r.h }, { x: r.x + r.w, y: r.y }],
    [{ x: r.x + r.w, y: r.y + r.h }, { x: r.x, y: r.y }],
  ];
  const corner = corners.find(([c]) => Math.hypot(c.x - p.x, c.y - p.y) <= 12 / zoom);
  // Inside moves the rect, unless it's the whole image (nothing to move): then a drag draws a new one.
  const within = (r.w < W || r.h < H) && p.x > r.x && p.x < r.x + r.w && p.y > r.y && p.y < r.y + r.h;
  // cropFromDrag expects its start inside the image.
  const start = { x: Math.max(0, Math.min(W, p.x)), y: Math.max(0, Math.min(H, p.y)) };
  drag = corner ? { kind: 'crop-corner', fixed: corner[1] } : within ? { kind: 'crop-move', start: p, orig: r } : { kind: 'crop-new', start, prev: r };
}

function moveCropDrag(p: M.Pt) {
  if (!crop || !drag) return;
  const ratio = crop.ratio === 'free' ? null : M.RATIOS[crop.ratio];
  if (drag.kind === 'crop-move') {
    const o = drag.orig;
    crop.draft = { ...o, x: Math.max(0, Math.min(W - o.w, o.x + p.x - drag.start.x)), y: Math.max(0, Math.min(H - o.h, o.y + p.y - drag.start.y)) };
  } else if (drag.kind === 'crop-new') crop.draft = M.cropFromDrag(drag.start, p, ratio, W, H);
  else if (drag.kind === 'crop-corner') crop.draft = M.cropFromDrag(drag.fixed, p, ratio, W, H);
  render();
}

$('#ratios').addEventListener('click', (e) => {
  const b = (e.target as Element).closest<HTMLElement>('[data-ratio]');
  if (!b || !crop) return;
  crop.ratio = b.dataset.ratio as M.Ratio;
  if (crop.ratio !== 'free') crop.draft = M.fitRatio(crop.draft, M.RATIOS[crop.ratio]);
  render();
});
$('#crop-apply').addEventListener('click', () => exitCrop(true));
$('#crop-cancel').addEventListener('click', () => exitCrop(false));

// ---------- save / copy / close / delete ----------

function ask(text: string, buttons: { label: string; value: string; primary?: boolean; danger?: boolean }[]): Promise<string> {
  return new Promise((resolve) => {
    $('#modal-text').textContent = text;
    const acts = $('#modal-acts');
    acts.replaceChildren(
      ...buttons.map((b) => {
        const el = document.createElement('button');
        el.className = b.primary ? 'b1' : 'b2';
        if (b.danger) el.classList.add('danger');
        el.textContent = b.label;
        el.onclick = () => {
          $('#modal').hidden = true;
          resolve(b.value);
        };
        return el;
      }),
    );
    $('#modal').hidden = false;
    // Enter picks the safe choice: Cancel on a destructive prompt, else the primary action.
    (acts.querySelector<HTMLElement>('.b1:not(.danger)') ?? (acts.firstElementChild as HTMLElement | null))?.focus();
  });
}

const fail = (what: string, e: unknown) => ask(`${what}: ${e}`, [{ label: 'OK', value: 'ok', primary: true }]);

/** True when what's on screen is now on disk (an edit made during the write stays unsaved). */
async function save(): Promise<boolean> {
  const d = hist.doc;
  let copied: boolean;
  try {
    copied = await ipc.saveImage(await exportPng(img, d, u, W, H));
  } catch (e) {
    await fail("Couldn't save", e);
    return false;
  }
  hist.markSaved(d);
  render();
  // The file is written either way: a clipboard failure only offers a retry (spec §4).
  while (!copied && (await ask("Saved, but couldn't copy to the clipboard.", [{ label: 'OK', value: 'ok' }, { label: 'Retry', value: 'retry', primary: true }])) === 'retry') {
    copied = await ipc.retryCopy(info.path).then(() => true, () => false);
  }
  return hist.doc === d;
}

/** Saves if needed (typed text and a crop draft included), then puts path + file + image on the clipboard again. */
async function copy(): Promise<boolean> {
  commitText();
  if (crop) exitCrop(true);
  if (hist.dirty) return save();
  try {
    await ipc.retryCopy(info.path);
    return true;
  } catch (e) {
    await fail("Couldn't copy", e);
    return false;
  }
}

async function done() {
  if (busy) return;
  busy = true;
  if (await copy()) await ipc.closeWindow();
  busy = false;
}

async function requestClose() {
  commitText();
  if (!hist.dirty) return ipc.closeWindow();
  const r = await ask(`Save changes to ${info.name}?`, [
    { label: 'Discard', value: 'discard', danger: true },
    { label: 'Cancel', value: 'cancel' },
    { label: 'Save', value: 'save', primary: true },
  ]);
  if (r === 'discard') await ipc.closeWindow();
  else if (r === 'save' && (await save())) await ipc.closeWindow();
}

async function remove() {
  const r = await ask(`Delete ${info.name}? This can't be undone.`, [
    { label: 'Cancel', value: 'cancel' },
    { label: 'Delete', value: 'delete', primary: true, danger: true },
  ]);
  if (r === 'delete') await ipc.editorDelete().catch((e) => fail("Couldn't delete", e));
}

// ---------- chrome ----------

function undo() {
  if (hist.undo()) selectedId = null;
  render();
}

function redo() {
  if (hist.redo()) selectedId = null;
  render();
}

$('#undo').addEventListener('click', undo);
$('#redo').addEventListener('click', redo);
$('#delete').addEventListener('click', () => void remove());
$('#copy').addEventListener('click', () => void copy());
$('#done').addEventListener('click', () => void done());
$('#close').addEventListener('click', () => void requestClose());
$('#copy-path').addEventListener('click', () => void ipc.copyPath(info.path));
$('#reveal').addEventListener('click', () => void ipc.revealCapture(info.path));
$('#zoom-in').addEventListener('click', () => setZoom(M.zoomStep(zoom, 1)));
$('#zoom-out').addEventListener('click', () => setZoom(M.zoomStep(zoom, -1)));
$('#fit').addEventListener('click', () => {
  fit();
  render();
});
$('#rail').addEventListener('click', (e) => {
  const b = (e.target as Element).closest<HTMLElement>('button');
  if (!b) return;
  if (b.dataset.tool) setTool(b.dataset.tool as Tool);
  else if (b.id === 'swatch') {
    const s = selected();
    const t = s ? (s.kind as Tool) : tool;
    openPopover(SECTIONS[t].length ? t : 'arrow', b);
  }
});

const KEYS: Record<string, Tool> = { v: 'select', c: 'crop', a: 'arrow', r: 'rect', o: 'ellipse', l: 'line', p: 'pen', h: 'highlight', t: 'text', n: 'counter', b: 'redact' };

/** Handles `k` as the pressed key; false when it means nothing here. */
function shortcut(e: KeyboardEvent, k: string): boolean {
  const mod = e.ctrlKey || e.metaKey;
  if (mod && k === 'z') {
    e.preventDefault();
    if (e.shiftKey) redo();
    else undo();
  } else if (mod && k === 'y') {
    e.preventDefault();
    redo();
  } else if (mod && k === 'c') {
    e.preventDefault();
    void copy();
  } else if (mod && k === 's') {
    e.preventDefault();
    void done();
  } else if (mod && (k === '=' || k === '+')) {
    e.preventDefault();
    setZoom(M.zoomStep(zoom, 1));
  } else if (mod && k === '-') {
    e.preventDefault();
    setZoom(M.zoomStep(zoom, -1));
  } else if (mod && k === '0') {
    e.preventDefault();
    fit();
    render();
  } else if (mod && k === '1') {
    e.preventDefault();
    setZoom(1);
  } else if (e.key === 'Enter') {
    e.preventDefault();
    if (crop) exitCrop(true);
    else void done();
  } else if (e.key === 'Escape') {
    if (popoverOpen()) closePopover();
    else if (crop) exitCrop(false);
    else if (selectedId !== null) {
      selectedId = null;
      render();
    } else void requestClose();
  } else if ((e.key === 'Delete' || e.key === 'Backspace') && selectedId !== null) {
    commit({ ...hist.doc, shapes: hist.doc.shapes.filter((s) => s.id !== selectedId) });
    selectedId = null;
    render();
  } else if (e.key.startsWith('Arrow') && selectedId !== null) {
    e.preventDefault();
    const d = e.shiftKey ? 10 : 1;
    const step: Record<string, [number, number]> = { ArrowLeft: [-d, 0], ArrowRight: [d, 0], ArrowUp: [0, -d], ArrowDown: [0, d] };
    const s = selected();
    const [dx, dy] = step[e.key] ?? [0, 0];
    if (s) replaceShape(M.translate(s, dx, dy));
  } else if (!mod && !e.altKey && KEYS[k]) setTool(KEYS[k]);
  else return false;
  return true;
}

addEventListener('keydown', (e) => {
  if (editingText || !$('#modal').hidden) return;
  if (!shortcut(e, e.key.toLowerCase())) {
    const k = M.usKey(e);
    if (k) shortcut(e, k);
  }
});

fit();
render();
ready = true;
