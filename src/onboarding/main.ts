import '../shared/base';
import '../shared/glass.css';
import { mountIcons } from '../shared/icons';
import * as ipc from '../shared/ipc';

mountIcons();
const s = await ipc.getSettings();
const rows: [string, string][] = [
  ['Capture area (with toolbar)', s.shortcuts.area],
  ['Capture screen', s.shortcuts.screen],
  ['Capture window', s.shortcuts.window],
  ['Record screen', s.shortcuts.record],
];
// Taken too, and not rebindable.
const wired = (
  {
    windows: ['Capture area (PrtScn)', 'Print'],
    macos: ['Capture options (⌘⇧5)', 'Super+Shift+5'],
  } as Record<string, [string, string]>
)[s.platform];
if (wired) rows.splice(1, 0, wired);
const keys = document.querySelector<HTMLElement>('#keys')!;
for (const [label, key] of rows) {
  const row = document.createElement('div');
  const kbd = document.createElement('kbd');
  kbd.textContent = key || 'Not set'; // from config.toml: never HTML
  row.append(label, kbd);
  keys.append(row);
}
const err = document.querySelector<HTMLElement>('#err')!;

/** macOS: capturing needs the Screen Recording permission, granted in System Settings. */
function permissionStep() {
  document.querySelector('#title')!.textContent = 'Allow screen recording';
  document.querySelector('#text')!.textContent =
    'macOS asks once. Turn on rshot in System Settings → Privacy & Security → Screen Recording, then quit and reopen rshot.';
  keys.hidden = true;
  document.querySelector<HTMLElement>('#choice')!.hidden = true;
  document.querySelector<HTMLElement>('#perm')!.hidden = false;
}
document.querySelector('#open')!.addEventListener('click', () => void ipc.requestScreenPermission());
document.querySelector('#done')!.addEventListener('click', () => void ipc.closeWindow());

document.querySelector('#no')!.addEventListener('click', async () => {
  try {
    await ipc.onboardingChoice(false);
    await ipc.closeWindow();
  } catch (e) {
    err.hidden = false;
    err.textContent = `Couldn't save your choice: ${String(e)}`;
  }
});

const yes = document.querySelector<HTMLButtonElement>('#yes')!;
// Once: after a failed takeover the button only closes the window, it never retries.
yes.addEventListener('click', async () => {
  try {
    await ipc.onboardingChoice(true);
    if (s.platform === 'macos' && !s.screen_permission) permissionStep();
    else await ipc.closeWindow();
  } catch (e) {
    yes.textContent = 'Close';
    yes.onclick = () => void ipc.closeWindow();
    const manual = (await ipc.getSettings()).manual;
    keys.hidden = true; // the error lists the keys with their commands
    err.hidden = false;
    err.textContent = `Couldn't take over the shortcuts: ${String(e)}. Bind these yourself:`;
    for (const [k, c] of manual) {
      const code = document.createElement('code');
      code.textContent = `${k} → ${c}`;
      err.appendChild(code);
    }
  }
}, { once: true });
