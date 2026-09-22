import '../shared/glass.css';
import './overlay.css';
import { listen } from '@tauri-apps/api/event';
import * as ipc from '../shared/ipc';

// Minimal overlay for the Task 6 performance gate; Task 7 replaces this file.
const canvas = document.querySelector<HTMLCanvasElement>('#frame')!;
let info: ipc.OverlayInfo | null = null;

// Keep: non-active overlays wait for overlay:primary-ready (Task 6 perf gate)
let primaryReady = 0;
let wakeWaiter = () => {};
void listen<number>('overlay:primary-ready', (e) => {
  primaryReady = e.payload;
  wakeWaiter();
});
/** Resolves once the active overlay has painted this session's frame (or after 500 ms). */
function afterPrimary(token: number): Promise<void> {
  if (primaryReady === token) return Promise.resolve();
  return new Promise((resolve) => {
    wakeWaiter = () => primaryReady === token && resolve();
    setTimeout(resolve, 500);
  });
}

async function load() {
  const next = await ipc.overlayInfo();
  if (!next) return;
  if (!next.active) await afterPrimary(next.token);
  const buf = await ipc.overlayFrame();
  info = next;
  canvas.width = next.width;
  canvas.height = next.height;
  canvas.getContext('2d')!.putImageData(new ImageData(new Uint8ClampedArray(buf), next.width, next.height), 0, 0);
  await ipc.overlayReady(next.token);
}

addEventListener('keydown', (e) => {
  if (!info) return;
  if (e.key === 'Escape') void ipc.overlayCancel();
  if (e.key === 'Enter') void ipc.overlayCapture(info.token, { kind: 'screen' });
});
void listen('overlay:show', () => void load());
void listen('overlay:hide', () => {
  info = null;
  canvas.width = canvas.height = 0;
});
void load();
