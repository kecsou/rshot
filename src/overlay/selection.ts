/** Pure geometry for the overlay. All values are image pixels of one monitor frame. */
export type Rect = { x: number; y: number; w: number; h: number };
export type Handle = 'n' | 's' | 'e' | 'w' | 'ne' | 'nw' | 'se' | 'sw';

/** Rect spanned by a drag from (ax, ay) to (bx, by); `square` keeps it 1:1 from the anchor. */
export function fromPoints(ax: number, ay: number, bx: number, by: number, square = false): Rect {
  let w = bx - ax;
  let h = by - ay;
  if (square) {
    const s = Math.max(Math.abs(w), Math.abs(h));
    w = Math.sign(w || 1) * s;
    h = Math.sign(h || 1) * s;
  }
  return { x: Math.min(ax, ax + w), y: Math.min(ay, ay + h), w: Math.abs(w), h: Math.abs(h) };
}

export function clamp(r: Rect, W: number, H: number): Rect {
  const x = Math.max(0, Math.min(r.x, W));
  const y = Math.max(0, Math.min(r.y, H));
  return { x, y, w: Math.max(0, Math.min(r.x + r.w, W) - x), h: Math.max(0, Math.min(r.y + r.h, H) - y) };
}

export function move(r: Rect, dx: number, dy: number, W: number, H: number): Rect {
  return { ...r, x: Math.max(0, Math.min(r.x + dx, W - r.w)), y: Math.max(0, Math.min(r.y + dy, H - r.h)) };
}

/** Moves the edges named by `h` to the pointer, then normalizes (edges may cross). */
export function resize(r: Rect, h: Handle, px: number, py: number): Rect {
  let x0 = r.x;
  let y0 = r.y;
  let x1 = r.x + r.w;
  let y1 = r.y + r.h;
  if (h.includes('w')) x0 = px;
  if (h.includes('e')) x1 = px;
  if (h.includes('n')) y0 = py;
  if (h.includes('s')) y1 = py;
  return fromPoints(x0, y0, x1, y1);
}

export function handleAt(r: Rect, px: number, py: number, tol: number): Handle | 'inside' | null {
  const near = (a: number, b: number) => Math.abs(a - b) <= tol;
  if (px < r.x - tol || px > r.x + r.w + tol || py < r.y - tol || py > r.y + r.h + tol) return null;
  const v = near(py, r.y) ? 'n' : near(py, r.y + r.h) ? 's' : '';
  const hz = near(px, r.x) ? 'w' : near(px, r.x + r.w) ? 'e' : '';
  if (v || hz) return (v + hz) as Handle;
  return px > r.x && px < r.x + r.w && py > r.y && py < r.y + r.h ? 'inside' : null;
}

/** First window containing the point; callers pass windows top-most first. */
export function windowAt<T extends Rect>(wins: T[], px: number, py: number): T | null {
  return wins.find((w) => px >= w.x && py >= w.y && px < w.x + w.w && py < w.y + w.h) ?? null;
}
