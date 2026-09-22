import { describe, expect, it } from 'vitest';
import { clamp, fromPoints, handleAt, move, resize, windowAt } from './selection';

describe('fromPoints', () => {
  it('normalizes a drag up-left', () => expect(fromPoints(100, 80, 40, 20)).toEqual({ x: 40, y: 20, w: 60, h: 60 }));
  it('squares from the anchor', () => expect(fromPoints(10, 10, 50, 30, true)).toEqual({ x: 10, y: 10, w: 40, h: 40 }));
  it('squares up-left', () => expect(fromPoints(50, 50, 40, 20, true)).toEqual({ x: 20, y: 20, w: 30, h: 30 }));
});

describe('clamp', () => {
  it('clips to the frame', () => expect(clamp({ x: -10, y: 90, w: 50, h: 50 }, 100, 100)).toEqual({ x: 0, y: 90, w: 40, h: 10 }));
});

describe('move', () => {
  it('keeps the rect inside the frame', () => {
    expect(move({ x: 10, y: 10, w: 20, h: 20 }, 100, -50, 100, 100)).toEqual({ x: 80, y: 0, w: 20, h: 20 });
  });
});

describe('resize', () => {
  const r = { x: 10, y: 10, w: 40, h: 30 };
  it('drags the south-east corner', () => expect(resize(r, 'se', 70, 60)).toEqual({ x: 10, y: 10, w: 60, h: 50 }));
  it('drags the west edge past the east edge and flips', () => expect(resize(r, 'w', 80, 999)).toEqual({ x: 50, y: 10, w: 30, h: 30 }));
});

describe('handleAt', () => {
  const r = { x: 100, y: 100, w: 200, h: 100 };
  it('finds corners and edges within tolerance', () => {
    expect(handleAt(r, 103, 98, 8)).toBe('nw');
    expect(handleAt(r, 200, 205, 8)).toBe('s');
    expect(handleAt(r, 297, 150, 8)).toBe('e');
  });
  it('reports inside and outside', () => {
    expect(handleAt(r, 200, 150, 8)).toBe('inside');
    expect(handleAt(r, 50, 50, 8)).toBeNull();
  });
});

describe('windowAt', () => {
  it('returns the first (top-most) window containing the point', () => {
    const wins = [{ id: 2, x: 50, y: 50, w: 100, h: 100 }, { id: 1, x: 0, y: 0, w: 500, h: 500 }];
    expect(windowAt(wins, 60, 60)?.id).toBe(2);
    expect(windowAt(wins, 10, 10)?.id).toBe(1);
    expect(windowAt(wins, 600, 10)).toBeNull();
  });
});
