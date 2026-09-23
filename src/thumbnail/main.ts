import '../shared/base';
import '../shared/glass.css';
import './thumbnail.css';
import { startDrag } from '@crabnebula/tauri-plugin-drag';
import { mountIcons } from '../shared/icons';
import * as ipc from '../shared/ipc';

mountIcons();
const card = document.querySelector<HTMLElement>('#card')!;
const img = document.querySelector<HTMLImageElement>('#img')!;
const DISMISS_MS = 5000;

function setCopied(ok: boolean) {
  document.querySelector('#status')!.textContent = ok ? 'Path copied' : 'Copy failed';
  document.querySelector('#ok')!.classList.toggle('fail', !ok);
  document.querySelector<HTMLElement>('#retry')!.hidden = ok;
}

/** Small PNG for the drag cursor (the full capture would be enormous). */
function dragIcon(): string {
  const w = 160;
  const h = Math.max(1, Math.round((img.naturalHeight / Math.max(1, img.naturalWidth)) * w));
  const c = document.createElement('canvas');
  c.width = w;
  c.height = h;
  c.getContext('2d')!.drawImage(img, 0, 0, w, h);
  return c.toDataURL('image/png');
}

async function run() {
  const t = await ipc.thumbnailInfo();
  if (!t) return void ipc.dismissThumbnail();
  img.src = URL.createObjectURL(new Blob([await ipc.readCapture(t.path)], { type: 'image/png' }));
  document.querySelector('#path')!.textContent = t.display;
  setCopied(t.copied);

  let left = DISMISS_MS;
  let last = performance.now();
  let hovering = false;
  let gone = false;
  const dismiss = () => {
    if (gone) return;
    gone = true;
    card.classList.add('out');
    setTimeout(() => void ipc.dismissThumbnail(), 200);
  };
  const tick = (now: number) => {
    if (!hovering) left -= now - last;
    last = now;
    if (left <= 0) dismiss();
    else if (!gone) requestAnimationFrame(tick);
  };
  requestAnimationFrame(tick);
  card.addEventListener('mouseenter', () => (hovering = true));
  card.addEventListener('mouseleave', () => (hovering = false));
  // The daemon closes this card once the editor is up; on failure it notifies and the card stays.
  const edit = () => ipc.openEditor(t.path).catch(() => {});

  card.addEventListener('click', async (e) => {
    const b = (e.target as Element).closest<HTMLButtonElement>('button');
    if (!b) return;
    if (b.dataset.act === 'edit') await edit();
    else if (b.dataset.act === 'reveal') await ipc.revealCapture(t.path);
    else if (b.dataset.act === 'delete') await ipc.deleteCapture(t.path);
    else if (b.id === 'retry') {
      await ipc.retryCopy(t.path).then(() => setCopied(true)).catch(() => setCopied(false));
    }
  });

  // Click → edit; drag right → swipe away; any other drag → drag the file into another app.
  card.addEventListener('pointerdown', (e) => {
    if (e.button !== 0 || (e.target as Element).closest('button')) return;
    const sx = e.clientX;
    const sy = e.clientY;
    let mode: 'none' | 'swipe' | 'file' = 'none';
    const onMove = (m: PointerEvent) => {
      const dx = m.clientX - sx;
      const dy = m.clientY - sy;
      if (mode === 'none' && Math.hypot(dx, dy) > 6) {
        mode = dx > 0 && Math.abs(dx) > Math.abs(dy) ? 'swipe' : 'file';
        if (mode === 'file') {
          cleanup();
          // The drag's pointer grab swallows the card's mouseleave: resume the timer when it ends.
          void startDrag({ item: [t.path], icon: dragIcon() }, () => (hovering = false)).catch(() => {});
          return;
        }
      }
      if (mode === 'swipe') card.style.transform = `translateX(${Math.max(0, dx)}px)`;
    };
    const onUp = (u: PointerEvent) => {
      cleanup();
      if (mode === 'swipe') {
        if (u.clientX - sx > 80) dismiss();
        else card.style.transform = '';
      } else if (mode === 'none') void edit();
    };
    const cleanup = () => {
      removeEventListener('pointermove', onMove);
      removeEventListener('pointerup', onUp);
    };
    addEventListener('pointermove', onMove);
    addEventListener('pointerup', onUp);
  });
}

// A card that failed to load has no timer or handlers: close it rather than strand it on top.
void run().catch(() => ipc.dismissThumbnail());
