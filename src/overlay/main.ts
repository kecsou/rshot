import '../shared/glass.css';
import './overlay.css';
import { listen } from '@tauri-apps/api/event';
import * as ipc from '../shared/ipc';

// Minimal overlay for the Task 6 performance gate; Task 7 replaces this file.
const canvas = document.querySelector<HTMLCanvasElement>('#frame')!;
let info: ipc.OverlayInfo | null = null;

async function load() {
  const next = await ipc.overlayInfo();
  if (!next) return;
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
