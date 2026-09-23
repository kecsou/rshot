import { convertFileSrc } from '@tauri-apps/api/core';

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
 * What the card fetches: the asset protocol's largest single range (1000 KiB). Recordings are
 * faststart MP4s, so it starts with the moov (duration) and then the first frame, the poster.
 * ponytail: the moov grows with length (about 0.5 KB per second); a recording so long that it
 * doesn't fit gets the card's "can't play" text, while its editor still plays it.
 */
export const POSTER_BYTES = 1000 * 1024;

/**
 * A blob: URL for a recording, fetched over the asset protocol (granted one file at a time).
 * WebKitGTK's player can't stream from it: its GStreamer source takes only http(s) and blob URIs
 * ("No URI handler implemented for asset"). `bytes`: only the first ones.
 * ponytail: the editor holds the whole file in memory while it's open; a range-serving source
 * (localhost HTTP, or fragmented MP4 + MSE) if hour-long recordings get edited.
 */
export async function videoUrl(path: string, bytes?: number): Promise<string> {
  const r = await fetch(convertFileSrc(path), bytes ? { headers: { Range: `bytes=0-${bytes - 1}` } } : {});
  if (!r.ok) throw new Error(`${r.status}`);
  return URL.createObjectURL(await r.blob());
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
