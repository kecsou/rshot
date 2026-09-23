import '../shared/base';
import '../shared/glass.css';
import './settings.css';
import { getVersion } from '@tauri-apps/api/app';
import { mountIcons } from '../shared/icons';
import * as ipc from '../shared/ipc';
import { comboFrom } from './keys';

mountIcons();
let s = await ipc.getSettings();
let rebinding: HTMLElement | null = null;
const micSel = document.querySelector<HTMLSelectElement>('#mic')!;

/** Starts recording a shortcut into `b`, or stops (null). Tells rshot only when that changes. */
function setRebinding(b: HTMLElement | null) {
  const changed = !rebinding !== !b;
  rebinding = b;
  return changed ? ipc.setRebinding(!!b) : Promise.resolve();
}

const esc = (t: string) => t.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]!);

function render() {
  document.querySelectorAll<HTMLElement>('[data-key]').forEach((b) => {
    const on = s[b.dataset.key as ipc.BoolSetting];
    b.classList.toggle('on', on);
    b.setAttribute('aria-checked', String(on));
  });
  document.querySelector('#folder span')!.textContent = s.screenshots_dir;
  document.querySelector('#rfolder span')!.textContent = s.recordings_dir;
  document.querySelectorAll<HTMLElement>('#fps button').forEach((b) => b.classList.toggle('on', Number(b.dataset.fps) === s.fps));
  micSel.value = s.mic ?? '';
  document.querySelectorAll<HTMLElement>('#clip button').forEach((b) => b.classList.toggle('on', b.dataset.v === s.clipboard_mode));
  document.querySelectorAll<HTMLElement>('[data-shortcut]').forEach((b) => {
    if (b === rebinding) {
      b.className = 'keys rebind';
      b.textContent = 'Press new shortcut…  Esc to cancel';
      return;
    }
    b.className = 'keys';
    const v = s.shortcuts[b.dataset.shortcut as keyof ipc.Shortcuts];
    b.innerHTML = v ? v.split('+').map((k) => `<kbd>${esc(k)}</kbd>`).join('') : '<kbd>Not set</kbd>';
  });
  const err = document.querySelector<HTMLElement>('#err')!;
  err.hidden = !s.takeover_error;
  if (s.takeover_error) {
    err.innerHTML =
      `<p>${esc(s.takeover_error)}</p><p>Bind these yourself in your desktop's keyboard settings:</p>` +
      s.manual.map(([k, c]) => `<div><kbd>${esc(k)}</kbd><code>${esc(c)}</code></div>`).join('');
  }
}

/** Applies one change to the latest settings: the overlay's options can change some while this window is open. */
async function update(change: (fresh: ipc.Settings) => void) {
  const fresh = await ipc.getSettings();
  change(fresh);
  s = await ipc.setSettings(fresh);
  render();
}

// A rebind never outlives the window's focus: rshot's keys would stay unguarded.
addEventListener('blur', () => {
  if (!rebinding) return;
  void setRebinding(null);
  render();
});

addEventListener('focus', async () => {
  if (rebinding) return;
  const error = s.takeover_error; // keep the manual-binding help while the user goes to set it up
  s = { ...(await ipc.getSettings()), takeover_error: error };
  render();
});

document.addEventListener('click', async (e) => {
  const b = (e.target as Element).closest<HTMLElement>('button');
  if (!b) return;
  if (b.dataset.key) {
    const k = b.dataset.key as ipc.BoolSetting;
    const on = !s[k]; // the opposite of what the user sees
    await update((f) => (f[k] = on));
  } else if (b.dataset.v) {
    const mode = b.dataset.v as ipc.ClipboardMode;
    await update((f) => (f.clipboard_mode = mode));
  } else if (b.dataset.fps) {
    const fps = Number(b.dataset.fps);
    await update((f) => (f.fps = fps));
  } else if (b.id === 'folder') {
    const d = await ipc.pickFolder();
    if (d) await update((f) => (f.screenshots_dir = d));
  } else if (b.id === 'rfolder') {
    const d = await ipc.pickFolder();
    if (d) await update((f) => (f.recordings_dir = d));
  } else if (b.dataset.shortcut) {
    void setRebinding(b);
    render();
  } else if (b.id === 'config') await ipc.openConfig();
  else if (b.id === 'close') {
    await setRebinding(null);
    await ipc.closeWindow();
  }
});

micSel.addEventListener('change', async () => {
  const mic = micSel.value || null;
  await update((f) => (f.mic = mic));
});
// Renders again once filled, so a configured mic shows selected; a failure must not stop this script.
void ipc
  .listMics()
  .then((ms) => {
    ms.forEach((m) => micSel.add(new Option(m.label, m.id)));
    if (s.mic && !ms.some((m) => m.id === s.mic)) micSel.add(new Option(`${s.mic} (unavailable)`, s.mic));
    render();
  })
  .catch(() => {});

addEventListener('keydown', async (e) => {
  if (!rebinding) return;
  e.preventDefault();
  if (e.key === 'Escape') {
    void setRebinding(null);
    return render();
  }
  const combo = comboFrom(e);
  if (!combo) return;
  const which = rebinding.dataset.shortcut as keyof ipc.Shortcuts;
  await setRebinding(null);
  await update((f) => (f.shortcuts[which] = combo));
});

document.querySelector('#version')!.textContent = `rshot ${await getVersion()}`;
render();
