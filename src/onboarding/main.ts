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
];
for (const [label, key] of rows) {
  const row = document.createElement('div');
  const kbd = document.createElement('kbd');
  kbd.textContent = key; // from config.toml: never HTML
  row.append(label, kbd);
  document.querySelector('#keys')!.append(row);
}
const err = document.querySelector<HTMLElement>('#err')!;

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
    await ipc.closeWindow();
  } catch (e) {
    yes.textContent = 'Close';
    yes.onclick = () => void ipc.closeWindow();
    const manual = (await ipc.getSettings()).manual;
    document.querySelector<HTMLElement>('#keys')!.hidden = true; // the error lists the keys with their commands
    err.hidden = false;
    err.textContent = `Couldn't take over the shortcuts: ${String(e)}. Bind these yourself:`;
    for (const [k, c] of manual) {
      const code = document.createElement('code');
      code.textContent = `${k} → ${c}`;
      err.appendChild(code);
    }
  }
}, { once: true });
