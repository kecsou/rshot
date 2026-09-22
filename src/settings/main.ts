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

const esc = (t: string) => t.replace(/[&<>"]/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;' })[c]!);

function render() {
  document.querySelectorAll<HTMLElement>('[data-key]').forEach((b) => {
    const on = s[b.dataset.key as ipc.BoolSetting];
    b.classList.toggle('on', on);
    b.setAttribute('aria-checked', String(on));
  });
  document.querySelector('#folder span')!.textContent = s.screenshots_dir;
  document.querySelectorAll<HTMLElement>('#clip button').forEach((b) => b.classList.toggle('on', b.dataset.v === s.clipboard_mode));
  document.querySelectorAll<HTMLElement>('[data-shortcut]').forEach((b) => {
    if (b === rebinding) {
      b.className = 'keys rebind';
      b.textContent = 'Press new shortcut…  Esc to cancel';
      return;
    }
    b.className = 'keys';
    b.innerHTML = s.shortcuts[b.dataset.shortcut as keyof ipc.Shortcuts]
      .split('+')
      .map((k) => `<kbd>${esc(k)}</kbd>`)
      .join('');
  });
  const err = document.querySelector<HTMLElement>('#err')!;
  err.hidden = !s.takeover_error;
  if (s.takeover_error) {
    err.innerHTML =
      `<p>${esc(s.takeover_error)}</p><p>Bind these yourself in your desktop's keyboard settings:</p>` +
      s.manual.map(([k, c]) => `<div><kbd>${esc(k)}</kbd><code>${esc(c)}</code></div>`).join('');
  }
}

async function save() {
  s = await ipc.setSettings(s);
  render();
}

document.addEventListener('click', async (e) => {
  const b = (e.target as Element).closest<HTMLElement>('button');
  if (!b) return;
  if (b.dataset.key) {
    const k = b.dataset.key as ipc.BoolSetting;
    s[k] = !s[k];
    await save();
  } else if (b.dataset.v) {
    s.clipboard_mode = b.dataset.v as ipc.ClipboardMode;
    await save();
  } else if (b.id === 'folder') {
    const d = await ipc.pickFolder();
    if (d) {
      s.screenshots_dir = d;
      await save();
    }
  } else if (b.dataset.shortcut) {
    rebinding = b;
    render();
  } else if (b.id === 'config') await ipc.openConfig();
  else if (b.id === 'close') await ipc.closeWindow();
});

addEventListener('keydown', async (e) => {
  if (!rebinding) return;
  e.preventDefault();
  if (e.key === 'Escape') {
    rebinding = null;
    return render();
  }
  const combo = comboFrom(e);
  if (!combo) return;
  s.shortcuts[rebinding.dataset.shortcut as keyof ipc.Shortcuts] = combo;
  rebinding = null;
  await save();
});

document.querySelector('#version')!.textContent = `rshot ${await getVersion()}`;
render();
