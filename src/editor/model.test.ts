import { describe, expect, it } from 'vitest';
import * as M from './model';

const arrow = (id: number, a: M.Pt, b: M.Pt): M.Shape => ({ id, kind: 'arrow', a, b, color: '#ff453a', width: 1, ends: 'one', shadow: true });
const rect = (id: number, a: M.Pt, b: M.Pt): M.Shape => ({ id, kind: 'rect', a, b, color: '#ff453a', width: 1 });
const counter = (id: number, n: number): M.Shape => ({ id, kind: 'counter', at: { x: 50, y: 50 }, n, color: '#ff453a' });
const redact = (id: number): M.Shape => ({ id, kind: 'redact', a: { x: 0, y: 0 }, b: { x: 10, y: 10 }, mode: 'pixelate', strength: 0.5 });
const doc = (...shapes: M.Shape[]): M.Doc => ({ shapes, crop: null });

describe('geometry', () => {
  it('unitFor scales from 1080p, capped at 2', () => {
    expect(M.unitFor(1920, 1080)).toBe(1);
    expect(M.unitFor(800, 600)).toBe(1);
    expect(M.unitFor(3840, 2160)).toBe(2);
  });
  it('norm and distToSeg', () => {
    expect(M.norm({ x: 10, y: 5 }, { x: 2, y: 9 })).toEqual({ x: 2, y: 5, w: 8, h: 4 });
    expect(M.distToSeg({ x: 5, y: 3 }, { x: 0, y: 0 }, { x: 10, y: 0 })).toBe(3);
    expect(M.distToSeg({ x: 13, y: 4 }, { x: 0, y: 0 }, { x: 10, y: 0 })).toBe(5);
  });
  it('bounds covers pen strokes and counters', () => {
    const pen: M.Shape = { id: 1, kind: 'pen', pts: [{ x: 5, y: 9 }, { x: 1, y: 2 }, { x: 8, y: 4 }], color: '#fff', width: 0 };
    expect(M.bounds(pen)).toEqual({ x: 1, y: 2, w: 7, h: 7 });
    expect(M.bounds(counter(2, 1), 2)).toEqual({ x: 50 - 26, y: 50 - 26, w: 52, h: 52 });
  });
  it('redactBlock stays coarse enough to hide text, scaled by the unit and growing with strength', () => {
    expect(M.redactBlock(0, 1)).toBeGreaterThanOrEqual(12);
    expect(M.redactBlock(0, 2)).toBeGreaterThanOrEqual(24);
    expect(M.redactBlock(0.5, 1)).toBeGreaterThan(M.redactBlock(0, 1));
    expect(M.redactBlock(1, 1)).toBeGreaterThan(M.redactBlock(0.5, 1));
  });
});

describe('hit testing', () => {
  it('hits an arrow near its shaft only', () => {
    const s = arrow(1, { x: 0, y: 0 }, { x: 100, y: 0 });
    expect(M.hitShape(s, { x: 50, y: 4 }, 3)).toBe(true);
    expect(M.hitShape(s, { x: 50, y: 20 }, 3)).toBe(false);
  });
  it('hits a rectangle on its border, not its inside', () => {
    const s = rect(1, { x: 0, y: 0 }, { x: 100, y: 50 });
    expect(M.hitShape(s, { x: 1, y: 25 }, 3)).toBe(true);
    expect(M.hitShape(s, { x: 50, y: 25 }, 3)).toBe(false);
  });
  it('hits an ellipse near its outline only, even when elongated', () => {
    const s: M.Shape = { id: 1, kind: 'ellipse', a: { x: 0, y: 0 }, b: { x: 400, y: 40 }, color: '#fff', width: 1 };
    expect(M.hitShape(s, { x: 200, y: 1 }, 3)).toBe(true);
    expect(M.hitShape(s, { x: 430, y: 20 }, 3)).toBe(false); // 30 px past the tip
    expect(M.hitShape(s, { x: 350, y: 20 }, 3)).toBe(false); // inside, ~13 px from the stroke
    const flat: M.Shape = { ...s, b: { x: 400, y: 0 } };
    expect(M.hitShape(flat, { x: 200, y: 2 }, 3)).toBe(true);
  });
  it('hits a pen stroke near any segment', () => {
    const s: M.Shape = { id: 1, kind: 'pen', pts: [{ x: 0, y: 0 }, { x: 10, y: 0 }, { x: 10, y: 10 }], color: '#fff', width: 0 };
    expect(M.hitShape(s, { x: 5, y: 1 }, 3)).toBe(true);
    expect(M.hitShape(s, { x: 11, y: 8 }, 3)).toBe(true);
    expect(M.hitShape(s, { x: 5, y: 5 }, 3)).toBe(false);
  });
  it('hitTest returns the top-most shape', () => {
    const d = doc(rect(1, { x: 0, y: 0 }, { x: 100, y: 100 }), arrow(2, { x: 0, y: 0 }, { x: 100, y: 0 }));
    expect(M.hitTest(d, { x: 50, y: 1 }, 3)?.id).toBe(2);
    expect(M.hitTest(d, { x: 1, y: 50 }, 3)?.id).toBe(1);
    expect(M.hitTest(d, { x: 50, y: 50 }, 3)).toBeNull();
  });
});

describe('editing', () => {
  it('translate moves every point', () => {
    expect(M.translate(arrow(1, { x: 0, y: 0 }, { x: 10, y: 10 }), 5, -2)).toMatchObject({ a: { x: 5, y: -2 }, b: { x: 15, y: 8 } });
  });
  it('handles gives normalised corners for boxes', () => {
    expect(M.handles(rect(1, { x: 50, y: 10 }, { x: 10, y: 40 }))).toEqual([
      { id: 'nw', p: { x: 10, y: 10 } },
      { id: 'ne', p: { x: 50, y: 10 } },
      { id: 'sw', p: { x: 10, y: 40 } },
      { id: 'se', p: { x: 50, y: 40 } },
    ]);
  });
  it('dragHandle keeps the opposite corner fixed', () => {
    const s = rect(1, { x: 10, y: 10 }, { x: 50, y: 40 });
    expect(M.bounds(M.dragHandle(s, 'nw', { x: 0, y: 0 }))).toEqual({ x: 0, y: 0, w: 50, h: 40 });
    expect(M.bounds(M.dragHandle(s, 'se', { x: 60, y: 60 }))).toEqual({ x: 10, y: 10, w: 50, h: 50 });
    expect(M.dragHandle(arrow(2, { x: 0, y: 0 }, { x: 5, y: 5 }), 'b', { x: 9, y: 1 })).toMatchObject({ b: { x: 9, y: 1 } });
  });
  it('snap45 and squareEnd constrain with Shift', () => {
    expect(M.snap45({ x: 0, y: 0 }, { x: 10, y: 1 })).toEqual({ x: 10, y: 0 });
    expect(M.snap45({ x: 0, y: 0 }, { x: 10, y: 9 })).toEqual({ x: 10, y: 10 });
    expect(M.squareEnd({ x: 0, y: 0 }, { x: -3, y: 8 })).toEqual({ x: -8, y: 8 });
  });
  it('nextCounter continues after the highest number', () => {
    expect(M.nextCounter(doc())).toBe(1);
    expect(M.nextCounter(doc(counter(1, 1), counter(2, 4)))).toBe(5);
  });
  it('exportOrder paints redactions first, keeping the rest in z-order', () => {
    const d = doc(arrow(1, { x: 0, y: 0 }, { x: 1, y: 1 }), redact(2), rect(3, { x: 0, y: 0 }, { x: 1, y: 1 }));
    expect(M.exportOrder(d).map((s) => s.id)).toEqual([2, 1, 3]);
  });
});

describe('History', () => {
  it('undoes, redoes and tracks dirtiness against the saved snapshot', () => {
    const h = new M.History(doc());
    expect(h.dirty).toBe(false);
    h.commit(doc(counter(1, 1)));
    h.commit(doc(counter(1, 1), counter(2, 2)));
    expect(h.dirty).toBe(true);
    expect(h.undo()).toBe(true);
    expect(h.doc.shapes).toHaveLength(1);
    expect(h.redo()).toBe(true);
    expect(h.doc.shapes).toHaveLength(2);
    h.markSaved();
    expect(h.dirty).toBe(false);
    h.undo();
    h.commit(doc());
    expect(h.canRedo).toBe(false);
    expect(new M.History(doc()).undo()).toBe(false);
  });
  it('canUndo follows the stack; undoing back to the saved snapshot clears dirty', () => {
    const h = new M.History(doc());
    expect(h.canUndo).toBe(false);
    h.commit(doc(counter(1, 1)));
    expect(h.canUndo).toBe(true);
    h.markSaved();
    h.commit(doc(counter(1, 1), counter(2, 2)));
    expect(h.dirty).toBe(true);
    h.undo();
    expect(h.dirty).toBe(false);
    h.undo();
    expect(h.canUndo).toBe(false);
    expect(h.dirty).toBe(true);
  });
});

describe('zoom and crop', () => {
  it('fitZoom never upscales', () => {
    expect(M.fitZoom(1000, 500, 500, 500)).toBe(0.5);
    expect(M.fitZoom(100, 100, 500, 500)).toBe(1);
  });
  it('zoomStep walks the preset ladder', () => {
    expect(M.zoomStep(1, 1)).toBe(1.25);
    expect(M.zoomStep(0.8, -1)).toBe(0.75);
    expect(M.zoomStep(4, 1)).toBe(4);
  });
  it('zoomStep never reverses direction off the ends of the ladder', () => {
    expect(M.zoomStep(0.08, -1)).toBeLessThanOrEqual(0.08);
    expect(M.zoomStep(0.08, 1)).toBe(0.1);
    expect(M.zoomStep(5, 1)).toBeGreaterThanOrEqual(5);
    expect(M.zoomStep(5, -1)).toBe(4);
  });
  it('fitRatio centres the largest rect of that ratio', () => {
    expect(M.fitRatio({ x: 0, y: 0, w: 200, h: 100 }, 1)).toEqual({ x: 50, y: 0, w: 100, h: 100 });
  });
  it('cropFromDrag constrains to the ratio and to the image', () => {
    expect(M.cropFromDrag({ x: 0, y: 0 }, { x: 160, y: 200 }, 16 / 9, 1000, 1000)).toEqual({ x: 0, y: 0, w: 160, h: 90 });
    expect(M.cropFromDrag({ x: 10, y: 10 }, { x: 2000, y: 50 }, null, 100, 100)).toEqual({ x: 10, y: 10, w: 90, h: 40 });
  });
  it('viewToImage undoes zoom and crop offset', () => {
    expect(M.viewToImage({ x: 50, y: 20 }, 0.5, { x: 100, y: 10, w: 1, h: 1 })).toEqual({ x: 200, y: 50 });
  });
});
