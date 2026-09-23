export type Pt = { x: number; y: number };
export type Rect = { x: number; y: number; w: number; h: number };
export type Width = 0 | 1 | 2;
export type TextSize = 'S' | 'M' | 'L' | 'XL';
export type TextStyle = 'filled' | 'plain' | 'outline';
export type Ends = 'one' | 'both' | 'none';
export type RedactMode = 'pixelate' | 'blur' | 'solid';
export type HandleId = 'a' | 'b' | 'nw' | 'ne' | 'sw' | 'se';
export type Ratio = 'free' | '16:9' | '4:3' | '1:1';
type Base = { id: number };
type TwoPoint = Base & { a: Pt; b: Pt };
export type Shape =
  | (TwoPoint & { kind: 'arrow'; color: string; width: Width; ends: Ends; shadow: boolean })
  | (TwoPoint & { kind: 'line' | 'rect' | 'ellipse'; color: string; width: Width })
  | (TwoPoint & { kind: 'redact'; mode: RedactMode; strength: number })
  | (Base & { kind: 'pen' | 'highlight'; pts: Pt[]; color: string; width: Width })
  | (Base & { kind: 'text'; at: Pt; text: string; color: string; size: TextSize; style: TextStyle; w: number; h: number })
  | (Base & { kind: 'counter'; at: Pt; n: number; color: string });
export type Doc = { shapes: Shape[]; crop: Rect | null };

export const WIDTHS = [3, 5, 8] as const;
export const HIGHLIGHT_WIDTHS = [14, 22, 34] as const;
export const TEXT_PX: Record<TextSize, number> = { S: 14, M: 18, L: 26, XL: 36 };
export const COUNTER_R = 13;
export const SWATCHES = ['#ff453a', '#ff9f0a', '#ffd60a', '#30d158', '#0a84ff', '#bf5af2', '#ffffff', '#1c1c1e'];
export const ZOOMS = [0.1, 0.25, 0.33, 0.5, 0.67, 0.75, 1, 1.25, 1.5, 2, 3, 4];
export const RATIOS: Record<Exclude<Ratio, 'free'>, number> = { '16:9': 16 / 9, '4:3': 4 / 3, '1:1': 1 };

/** Annotation scale: 1 at 1080p, up to 2 on 4K-class captures. */
export function unitFor(w: number, h: number): number {
  return Math.min(2, Math.max(1, Math.min(w, h) / 1080));
}

/** Redaction block size in image px: coarse enough at any strength that text can't be read back, scaled with the unit. */
export function redactBlock(strength: number, u: number): number {
  return Math.round((12 + 20 * strength) * u);
}

/** Area a redaction covers: snapped outward to whole pixels (so antialiased edges can't blend
 *  original pixels back in) and clipped to the W×H image; null when nothing of it is left. */
export function redactRect(r: Rect, W: number, H: number): Rect | null {
  const x = Math.max(0, Math.floor(r.x));
  const y = Math.max(0, Math.floor(r.y));
  const w = Math.min(W, Math.ceil(r.x + r.w)) - x;
  const h = Math.min(H, Math.ceil(r.y + r.h)) - y;
  return w >= 1 && h >= 1 ? { x, y, w, h } : null;
}

export function norm(a: Pt, b: Pt): Rect {
  return { x: Math.min(a.x, b.x), y: Math.min(a.y, b.y), w: Math.abs(b.x - a.x), h: Math.abs(b.y - a.y) };
}

export function strokeWidth(s: Shape, u: number): number {
  if (s.kind === 'highlight') return HIGHLIGHT_WIDTHS[s.width] * u;
  return 'width' in s ? WIDTHS[s.width] * u : 0;
}

export function bounds(s: Shape, u = 1): Rect {
  switch (s.kind) {
    case 'pen':
    case 'highlight': {
      const xs = s.pts.map((p) => p.x);
      const ys = s.pts.map((p) => p.y);
      const x = Math.min(...xs);
      const y = Math.min(...ys);
      return { x, y, w: Math.max(...xs) - x, h: Math.max(...ys) - y };
    }
    case 'text':
      return { x: s.at.x, y: s.at.y, w: s.w, h: s.h };
    case 'counter': {
      const r = COUNTER_R * u;
      return { x: s.at.x - r, y: s.at.y - r, w: 2 * r, h: 2 * r };
    }
    default:
      return norm(s.a, s.b);
  }
}

export function distToSeg(p: Pt, a: Pt, b: Pt): number {
  const dx = b.x - a.x;
  const dy = b.y - a.y;
  const len2 = dx * dx + dy * dy;
  const t = len2 ? Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / len2)) : 0;
  return Math.hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy));
}

const inside = (r: Rect, p: Pt, pad: number) =>
  p.x >= r.x - pad && p.x <= r.x + r.w + pad && p.y >= r.y - pad && p.y <= r.y + r.h + pad;

const nearPath = (pts: Pt[], p: Pt, reach: number) =>
  pts.some((q, i) => distToSeg(p, pts[Math.max(0, i - 1)], q) <= reach);

/** Strokes hit near their line; rectangles and ellipses on their outline; the rest inside their box. */
export function hitShape(s: Shape, p: Pt, tol: number, u = 1): boolean {
  const reach = tol + strokeWidth(s, u) / 2;
  switch (s.kind) {
    case 'arrow':
    case 'line':
      return distToSeg(p, s.a, s.b) <= reach;
    case 'pen':
    case 'highlight':
      return nearPath(s.pts, p, reach);
    case 'rect': {
      const r = norm(s.a, s.b);
      const edge = Math.min(Math.abs(p.x - r.x), Math.abs(p.x - r.x - r.w), Math.abs(p.y - r.y), Math.abs(p.y - r.y - r.h));
      return inside(r, p, reach) && edge <= reach;
    }
    case 'ellipse': {
      const r = norm(s.a, s.b);
      const rx = r.w / 2;
      const ry = r.h / 2;
      // ponytail: 64-segment outline, error <= 0.12% of the radius; add samples if huge ellipses need sub-px hits.
      const outline = Array.from({ length: 65 }, (_, i) => {
        const t = (i / 64) * 2 * Math.PI;
        return { x: r.x + rx + rx * Math.cos(t), y: r.y + ry + ry * Math.sin(t) };
      });
      return nearPath(outline, p, reach);
    }
    default:
      return inside(bounds(s, u), p, tol);
  }
}

export function hitTest(doc: Doc, p: Pt, tol: number, u = 1): Shape | null {
  for (let i = doc.shapes.length - 1; i >= 0; i--) if (hitShape(doc.shapes[i], p, tol, u)) return doc.shapes[i];
  return null;
}

export function translate(s: Shape, dx: number, dy: number): Shape {
  const m = (p: Pt) => ({ x: p.x + dx, y: p.y + dy });
  switch (s.kind) {
    case 'pen':
    case 'highlight':
      return { ...s, pts: s.pts.map(m) };
    case 'text':
    case 'counter':
      return { ...s, at: m(s.at) };
    default:
      return { ...s, a: m(s.a), b: m(s.b) };
  }
}

export function handles(s: Shape): { id: HandleId; p: Pt }[] {
  if (s.kind === 'arrow' || s.kind === 'line') return [{ id: 'a', p: s.a }, { id: 'b', p: s.b }];
  if (s.kind === 'rect' || s.kind === 'ellipse' || s.kind === 'redact') {
    const r = norm(s.a, s.b);
    return [
      { id: 'nw', p: { x: r.x, y: r.y } },
      { id: 'ne', p: { x: r.x + r.w, y: r.y } },
      { id: 'sw', p: { x: r.x, y: r.y + r.h } },
      { id: 'se', p: { x: r.x + r.w, y: r.y + r.h } },
    ];
  }
  return [];
}

/** `constrain` (Shift): line ends snap to 45° and boxes to squares around the end that stays put. */
export function dragHandle(s: Shape, h: HandleId, p: Pt, constrain = false): Shape {
  if (s.kind === 'arrow' || s.kind === 'line') {
    const q = constrain ? snap45(h === 'a' ? s.b : s.a, p) : p;
    return h === 'a' ? { ...s, a: q } : { ...s, b: q };
  }
  if (s.kind === 'rect' || s.kind === 'ellipse' || s.kind === 'redact') {
    const r = norm(s.a, s.b);
    const opposite: Partial<Record<HandleId, Pt>> = {
      nw: { x: r.x + r.w, y: r.y + r.h },
      ne: { x: r.x, y: r.y + r.h },
      sw: { x: r.x + r.w, y: r.y },
      se: { x: r.x, y: r.y },
    };
    const fixed = opposite[h];
    return fixed ? { ...s, a: fixed, b: constrain ? squareEnd(fixed, p) : p } : s;
  }
  return s;
}

/** End point snapped to the nearest 45° (Shift on arrows and lines). */
export function snap45(a: Pt, b: Pt): Pt {
  const step = Math.PI / 4;
  const ang = Math.round(Math.atan2(b.y - a.y, b.x - a.x) / step) * step;
  const len = Math.hypot(b.x - a.x, b.y - a.y);
  return { x: a.x + Math.round(Math.cos(ang) * len), y: a.y + Math.round(Math.sin(ang) * len) };
}

/** End point giving a square/circle (Shift on rectangles, ellipses, redactions). */
export function squareEnd(a: Pt, b: Pt): Pt {
  const s = Math.max(Math.abs(b.x - a.x), Math.abs(b.y - a.y));
  return { x: a.x + Math.sign(b.x - a.x || 1) * s, y: a.y + Math.sign(b.y - a.y || 1) * s };
}

export function nextCounter(doc: Doc): number {
  return doc.shapes.reduce((n, s) => (s.kind === 'counter' ? Math.max(n, s.n) : n), 0) + 1;
}

/** Redactions read the original pixels, so they paint first; the rest keep z-order. */
export function exportOrder(doc: Doc): Shape[] {
  return [...doc.shapes.filter((s) => s.kind === 'redact'), ...doc.shapes.filter((s) => s.kind !== 'redact')];
}

export function fitZoom(w: number, h: number, vw: number, vh: number): number {
  return Math.min(1, vw / w, vh / h);
}

export function zoomStep(z: number, dir: 1 | -1): number {
  if (dir > 0) return ZOOMS.find((s) => s > z + 1e-6) ?? Math.max(z, ZOOMS[ZOOMS.length - 1]);
  return [...ZOOMS].reverse().find((s) => s < z - 1e-6) ?? Math.min(z, ZOOMS[0]);
}

/** Largest rect of `ratio` centred in `r`. */
export function fitRatio(r: Rect, ratio: number): Rect {
  const w = Math.min(r.w, r.h * ratio);
  const h = w / ratio;
  return { x: r.x + (r.w - w) / 2, y: r.y + (r.h - h) / 2, w, h };
}

/** Crop rect dragged from `a` to `b`, clipped to the image and held to `ratio` when set. */
export function cropFromDrag(a: Pt, b: Pt, ratio: number | null, W: number, H: number): Rect {
  const bx = Math.max(0, Math.min(W, b.x));
  const by = Math.max(0, Math.min(H, b.y));
  if (!ratio) return norm(a, { x: bx, y: by });
  let w = Math.abs(bx - a.x);
  let h = Math.abs(by - a.y);
  if (w / ratio > h) w = h * ratio;
  else h = w / ratio;
  return norm(a, { x: a.x + Math.sign(bx - a.x || 1) * w, y: a.y + Math.sign(by - a.y || 1) * h });
}

/** Shortcut fallback for keys that don't type a Latin letter (AZERTY digit row, Cyrillic): what the
 *  key at that spot types on a US layout (`Digit0` → 0, `KeyZ` → z, `Equal` → =). A typed letter is kept as is. */
export function usKey(e: { key: string; code: string }): string | undefined {
  if (/^[a-z]$/i.test(e.key)) return undefined;
  return /^(?:Key|Digit)(.)$/.exec(e.code)?.[1].toLowerCase() ?? { Equal: '=', Minus: '-' }[e.code];
}

export function viewToImage(p: Pt, zoom: number, crop: Rect | null): Pt {
  return { x: p.x / zoom + (crop?.x ?? 0), y: p.y / zoom + (crop?.y ?? 0) };
}

/** Undo/redo over immutable Doc snapshots; `dirty` compares against the last saved one. */
export class History {
  private past: Doc[] = [];
  private future: Doc[] = [];
  private saved: Doc;

  constructor(public doc: Doc) {
    this.saved = doc;
  }

  commit(next: Doc): void {
    this.past.push(this.doc);
    this.future = [];
    this.doc = next;
  }

  undo(): boolean {
    const prev = this.past.pop();
    if (!prev) return false;
    this.future.push(this.doc);
    this.doc = prev;
    return true;
  }

  redo(): boolean {
    const next = this.future.pop();
    if (!next) return false;
    this.past.push(this.doc);
    this.doc = next;
    return true;
  }

  /** `doc` = the snapshot that was written (an edit made during the write stays unsaved). */
  markSaved(doc = this.doc): void {
    this.saved = doc;
  }

  get canUndo() {
    return this.past.length > 0;
  }

  get canRedo() {
    return this.future.length > 0;
  }

  get dirty() {
    return this.doc !== this.saved;
  }
}
