/** m:ss.t (rounded first, so 59.96 is 1:00.0, not 0:60.0). */
export function fmt(s: number): string {
  const t = Math.round(s * 10) / 10;
  const m = Math.floor(t / 60);
  return `${m}:${(t - m * 60).toFixed(1).padStart(4, '0')}`;
}

/** m:ss, for the duration badge and the header. */
export function clock(s: number): string {
  const t = Math.round(s);
  return `${Math.floor(t / 60)}:${String(t % 60).padStart(2, '0')}`;
}

/** Keeps [start, end] inside [0, dur] and at least `minLen` long. */
export function clampTrim(start: number, end: number, dur: number, minLen: number): [number, number] {
  let s = Math.max(0, Math.min(start, dur));
  let e = Math.max(0, Math.min(end, dur));
  if (e - s < minLen) {
    if (s + minLen <= dur) e = s + minLen;
    else s = Math.max(0, e - minLen);
  }
  return [s, e];
}

export function timeAt(px: number, width: number, dur: number): number {
  return Math.max(0, Math.min(dur, (px / width) * dur));
}

/**
 * Resolves true once `v` knows its duration, false if it can't play here: an error (WebKitGTK
 * needs gstreamer1.0-libav for H.264), no usable duration, or nothing after `ms`. Never hangs.
 */
export function playable(v: HTMLVideoElement, ms = 10_000): Promise<boolean> {
  return new Promise((done) => {
    v.addEventListener('loadedmetadata', () => done(Number.isFinite(v.duration) && v.duration > 0), { once: true });
    v.addEventListener('error', () => done(false), { once: true });
    setTimeout(() => done(false), ms);
  });
}
