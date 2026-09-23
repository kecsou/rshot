import '../shared/base';
import '../shared/glass.css';
import './pill.css';
import { mountIcons } from '../shared/icons';
import * as ipc from '../shared/ipc';

mountIcons();
const $ = (s: string) => document.querySelector<HTMLElement>(s)!;
const time = $('#time');
const mic = $('#mic');
const bars = [...mic.querySelectorAll<HTMLElement>('i')];
const SHAPE = [0.4, 0.8, 0.55, 1, 0.3]; // each bar's share of the level, as in the mockup
const pad = (n: number) => String(Math.floor(n)).padStart(2, '0');

// Polled rather than counted here, so the timer can't drift from the tray's and the meter is live.
async function poll() {
  const info = await ipc.recordingInfo().catch(() => null);
  if (!info) return; // stopped: Rust closes this window
  const s = info.elapsed_ms / 1000;
  time.textContent = `${pad(s / 60)}:${pad(s % 60)}`;
  mic.hidden = !info.mic;
  if (info.level === null) return;
  const level = Math.min(1, Math.max(0, (info.level + 60) / 60)); // -60..0 dBFS
  bars.forEach((b, i) => (b.style.height = `${Math.max(2, Math.round(14 * level * SHAPE[i]))}px`));
}
void poll();
setInterval(() => void poll(), 100);

$('#stop').addEventListener('click', () => void ipc.recordingStop());
// Discard sits next to Stop: the first click only arms it (red), and a second one within 3 s discards.
const discard = $('#discard');
const arm = (on: boolean) => {
  discard.classList.toggle('armed', on);
  discard.title = on ? 'Click again to discard' : 'Discard recording';
  discard.setAttribute('aria-label', discard.title);
};
discard.addEventListener('click', () => {
  if (discard.classList.contains('armed')) return void ipc.recordingDiscard();
  arm(true);
  setTimeout(() => arm(false), 3000);
});
