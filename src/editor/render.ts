import { type Doc, type Rect, type Shape, COUNTER_R, TEXT_PX, bounds, exportOrder, handles, norm, redactBlock, strokeWidth } from './model';

export type Src = CanvasImageSource & { width: number; height: number };

const LIGHT = new Set(['#ffffff', '#ffd60a']);
/** Readable text colour on top of `color` (dark on white/yellow, white otherwise). */
export const ink = (color: string) => (LIGHT.has(color.toLowerCase()) ? '#1c1c1e' : '#ffffff');
const font = (px: number, weight = 600) => `${weight} ${px}px Inter, system-ui, sans-serif`;

/** Box a text annotation occupies (stored on the shape when the text is committed). */
export function measureText(ctx: CanvasRenderingContext2D, text: string, px: number, style: string): { w: number; h: number } {
  ctx.save();
  ctx.font = font(px);
  const w = Math.max(...text.split('\n').map((l) => ctx.measureText(l).width));
  ctx.restore();
  const padX = style === 'filled' ? px * 0.6 : 0;
  const padY = style === 'filled' ? px * 0.35 : 0;
  return { w: Math.ceil(w + padX * 2), h: Math.ceil(text.split('\n').length * px * 1.25 + padY * 2) };
}

function seg(ctx: CanvasRenderingContext2D, a: { x: number; y: number }, b: { x: number; y: number }) {
  ctx.beginPath();
  ctx.moveTo(a.x, a.y);
  ctx.lineTo(b.x, b.y);
  ctx.stroke();
}

function head(ctx: CanvasRenderingContext2D, tip: { x: number; y: number }, ang: number, size: number) {
  ctx.beginPath();
  ctx.moveTo(tip.x, tip.y);
  ctx.lineTo(tip.x - size * Math.cos(ang - 0.45), tip.y - size * Math.sin(ang - 0.45));
  ctx.lineTo(tip.x - size * Math.cos(ang + 0.45), tip.y - size * Math.sin(ang + 0.45));
  ctx.closePath();
  ctx.fill();
}

function arrow(ctx: CanvasRenderingContext2D, s: Extract<Shape, { kind: 'arrow' }>, u: number) {
  const w = strokeWidth(s, u);
  const size = Math.max(10 * u, w * 3.2);
  const ang = Math.atan2(s.b.y - s.a.y, s.b.x - s.a.x);
  const back = { x: Math.cos(ang) * size * 0.6, y: Math.sin(ang) * size * 0.6 };
  if (s.shadow) {
    ctx.shadowColor = 'rgba(0,0,0,.35)';
    ctx.shadowBlur = 4 * u;
    ctx.shadowOffsetY = 2 * u;
  }
  ctx.strokeStyle = ctx.fillStyle = s.color;
  ctx.lineWidth = w;
  // Shorten the shaft so its round cap doesn't poke through the head.
  const a = s.ends === 'both' ? { x: s.a.x + back.x, y: s.a.y + back.y } : s.a;
  const b = s.ends === 'none' ? s.b : { x: s.b.x - back.x, y: s.b.y - back.y };
  seg(ctx, a, b);
  if (s.ends !== 'none') head(ctx, s.b, ang, size);
  if (s.ends === 'both') head(ctx, s.a, ang + Math.PI, size);
}

function text(ctx: CanvasRenderingContext2D, s: Extract<Shape, { kind: 'text' }>, u: number) {
  const px = TEXT_PX[s.size] * u;
  const padX = s.style === 'filled' ? px * 0.6 : 0;
  const padY = s.style === 'filled' ? px * 0.35 : 0;
  ctx.font = font(px);
  ctx.textBaseline = 'top';
  if (s.style === 'filled') {
    ctx.fillStyle = s.color;
    ctx.beginPath();
    ctx.roundRect(s.at.x, s.at.y, s.w, s.h, px * 0.4);
    ctx.fill();
  }
  s.text.split('\n').forEach((line, i) => {
    const x = s.at.x + padX;
    const y = s.at.y + padY + i * px * 1.25 + px * 0.125;
    if (s.style === 'outline') {
      ctx.lineWidth = px * 0.22;
      ctx.strokeStyle = s.color;
      ctx.strokeText(line, x, y);
    }
    ctx.fillStyle = s.style === 'plain' ? s.color : ink(s.color);
    ctx.fillText(line, x, y);
  });
}

function counter(ctx: CanvasRenderingContext2D, s: Extract<Shape, { kind: 'counter' }>, u: number) {
  const r = COUNTER_R * u;
  ctx.shadowColor = 'rgba(0,0,0,.3)';
  ctx.shadowBlur = 4 * u;
  ctx.shadowOffsetY = 1.5 * u;
  ctx.fillStyle = s.color;
  ctx.beginPath();
  ctx.arc(s.at.x, s.at.y, r, 0, Math.PI * 2);
  ctx.fill();
  ctx.shadowColor = 'transparent';
  ctx.lineWidth = 2 * u;
  ctx.strokeStyle = '#ffffff';
  ctx.stroke();
  ctx.fillStyle = ink(s.color);
  ctx.font = font(Math.round(r * 1.05), 700);
  ctx.textAlign = 'center';
  ctx.textBaseline = 'middle';
  ctx.fillText(String(s.n), s.at.x, s.at.y + u);
}

/** Pixelate/blur re-sample the ORIGINAL pixels through a small canvas, so the saved PNG holds no trace of them. */
function redact(ctx: CanvasRenderingContext2D, src: Src, r: Rect, mode: string, strength: number, u: number) {
  // Snap outward to whole pixels so antialiased edges can't blend original pixels back in.
  const x = Math.floor(r.x), y = Math.floor(r.y);
  r = { x, y, w: Math.ceil(r.x + r.w) - x, h: Math.ceil(r.y + r.h) - y };
  if (r.w < 1 || r.h < 1) return;
  if (mode === 'solid') {
    ctx.fillStyle = '#1c1c1e';
    ctx.fillRect(r.x, r.y, r.w, r.h);
    return;
  }
  const block = redactBlock(strength, u);
  const small = document.createElement('canvas');
  small.width = Math.max(1, Math.round(r.w / block));
  small.height = Math.max(1, Math.round(r.h / block));
  const sc = small.getContext('2d')!;
  sc.imageSmoothingEnabled = true;
  sc.drawImage(src, r.x, r.y, r.w, r.h, 0, 0, small.width, small.height);
  ctx.imageSmoothingEnabled = mode === 'blur';
  ctx.imageSmoothingQuality = 'high';
  ctx.drawImage(small, 0, 0, small.width, small.height, r.x, r.y, r.w, r.h);
}

export function drawShape(ctx: CanvasRenderingContext2D, src: Src, s: Shape, u: number) {
  ctx.save();
  ctx.lineCap = 'round';
  ctx.lineJoin = 'round';
  switch (s.kind) {
    case 'redact':
      redact(ctx, src, norm(s.a, s.b), s.mode, s.strength, u);
      break;
    case 'arrow':
      arrow(ctx, s, u);
      break;
    case 'line':
      ctx.strokeStyle = s.color;
      ctx.lineWidth = strokeWidth(s, u);
      seg(ctx, s.a, s.b);
      break;
    case 'rect': {
      const r = norm(s.a, s.b);
      ctx.strokeStyle = s.color;
      ctx.lineWidth = strokeWidth(s, u);
      ctx.beginPath();
      ctx.roundRect(r.x, r.y, r.w, r.h, 6 * u);
      ctx.stroke();
      break;
    }
    case 'ellipse': {
      const r = norm(s.a, s.b);
      ctx.strokeStyle = s.color;
      ctx.lineWidth = strokeWidth(s, u);
      ctx.beginPath();
      ctx.ellipse(r.x + r.w / 2, r.y + r.h / 2, r.w / 2, r.h / 2, 0, 0, Math.PI * 2);
      ctx.stroke();
      break;
    }
    case 'pen':
    case 'highlight': {
      if (s.kind === 'highlight') ctx.globalAlpha = 0.4;
      ctx.strokeStyle = s.color;
      ctx.lineWidth = strokeWidth(s, u);
      ctx.beginPath();
      s.pts.forEach((p, i) => (i ? ctx.lineTo(p.x, p.y) : ctx.moveTo(p.x, p.y)));
      if (s.pts.length === 1) ctx.lineTo(s.pts[0].x + 0.01, s.pts[0].y);
      ctx.stroke();
      break;
    }
    case 'text':
      text(ctx, s, u);
      break;
    case 'counter':
      counter(ctx, s, u);
      break;
  }
  ctx.restore();
}

/** The capture plus its annotations in export order; `skipId` hides a text being edited. */
export function drawDoc(ctx: CanvasRenderingContext2D, src: Src, doc: Doc, u: number, skipId?: number) {
  ctx.drawImage(src, 0, 0);
  for (const s of exportOrder(doc)) if (s.id !== skipId) drawShape(ctx, src, s, u);
}

export function drawSelection(ctx: CanvasRenderingContext2D, s: Shape, zoom: number, u: number) {
  ctx.save();
  const hs = handles(s);
  if (!hs.length) {
    const b = bounds(s, u);
    const pad = 4 / zoom;
    ctx.setLineDash([4 / zoom, 3 / zoom]);
    ctx.strokeStyle = '#0a84ff';
    ctx.lineWidth = 1.5 / zoom;
    ctx.strokeRect(b.x - pad, b.y - pad, b.w + 2 * pad, b.h + 2 * pad);
  }
  for (const h of hs) {
    ctx.beginPath();
    ctx.arc(h.p.x, h.p.y, 5.5 / zoom, 0, Math.PI * 2);
    ctx.fillStyle = '#ffffff';
    ctx.fill();
    ctx.lineWidth = 2 / zoom;
    ctx.strokeStyle = '#0a84ff';
    ctx.stroke();
  }
  ctx.restore();
}

/** Crop mode: dim outside `r`, rule-of-thirds grid, corner brackets. */
export function drawCrop(ctx: CanvasRenderingContext2D, r: Rect, W: number, H: number, zoom: number) {
  ctx.save();
  ctx.fillStyle = 'rgba(0,0,0,.6)';
  ctx.beginPath();
  ctx.rect(0, 0, W, H);
  ctx.rect(r.x, r.y, r.w, r.h);
  ctx.fill('evenodd');
  ctx.strokeStyle = 'rgba(255,255,255,.4)';
  ctx.lineWidth = 1 / zoom;
  for (const t of [1 / 3, 2 / 3]) {
    seg(ctx, { x: r.x + r.w * t, y: r.y }, { x: r.x + r.w * t, y: r.y + r.h });
    seg(ctx, { x: r.x, y: r.y + r.h * t }, { x: r.x + r.w, y: r.y + r.h * t });
  }
  ctx.strokeStyle = '#ffffff';
  ctx.lineWidth = 1.5 / zoom;
  ctx.strokeRect(r.x, r.y, r.w, r.h);
  const L = 16 / zoom;
  ctx.lineWidth = 3 / zoom;
  ctx.lineCap = 'square';
  for (const [x, y, dx, dy] of [
    [r.x, r.y, 1, 1],
    [r.x + r.w, r.y, -1, 1],
    [r.x, r.y + r.h, 1, -1],
    [r.x + r.w, r.y + r.h, -1, -1],
  ]) {
    ctx.beginPath();
    ctx.moveTo(x + dx * L, y);
    ctx.lineTo(x, y);
    ctx.lineTo(x, y + dy * L);
    ctx.stroke();
  }
  ctx.restore();
}

/** Flattens `doc` over the original capture (cropped) into PNG bytes. */
export async function exportPng(src: Src, doc: Doc, u: number, W: number, H: number): Promise<Uint8Array> {
  const r = doc.crop ?? { x: 0, y: 0, w: W, h: H };
  const c = document.createElement('canvas');
  c.width = Math.round(r.x + r.w) - Math.round(r.x);
  c.height = Math.round(r.y + r.h) - Math.round(r.y);
  const ctx = c.getContext('2d')!;
  ctx.translate(-Math.round(r.x), -Math.round(r.y));
  drawDoc(ctx, src, doc, u);
  const blob = await new Promise<Blob>((ok, no) => c.toBlob((b) => (b ? ok(b) : no(new Error('PNG encoding failed'))), 'image/png'));
  return new Uint8Array(await blob.arrayBuffer());
}
