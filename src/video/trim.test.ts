import { describe, expect, it } from 'vitest';
import { clampTrim, clock, fmt, timeAt } from './trim';

describe('trim helpers', () => {
  it('formats m:ss.t', () => {
    expect(fmt(12.94)).toBe('0:12.9');
    expect(fmt(75)).toBe('1:15.0');
    expect(fmt(59.96)).toBe('1:00.0');
  });
  it('formats the m:ss badge, rounding before splitting', () => {
    expect(clock(34.2)).toBe('0:34');
    expect(clock(59.6)).toBe('1:00');
    expect(clock(125)).toBe('2:05');
  });
  it('keeps the range inside the video and at least minLen long', () => {
    expect(clampTrim(-1, 50, 34, 0.5)).toEqual([0, 34]);
    expect(clampTrim(10, 10.2, 34, 0.5)).toEqual([10, 10.5]);
    expect(clampTrim(33.9, 34, 34, 0.5)).toEqual([33.5, 34]);
  });
  it('maps pixels to seconds', () => {
    expect(timeAt(50, 200, 34)).toBe(8.5);
    expect(timeAt(-5, 200, 34)).toBe(0);
    expect(timeAt(500, 200, 34)).toBe(34);
  });
});
