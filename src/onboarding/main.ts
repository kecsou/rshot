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
document.querySelector('#keys')!.innerHTML = rows.map(([label, key]) => `<div>${label}<kbd>${key}</kbd></div>`).join('');

document.querySelector('#no')!.addEventListener('click', async () => {
  await ipc.onboardingChoice(false);
  await ipc.closeWindow();
});

document.querySelector('#yes')!.addEventListener('click', async () => {
  try {
    await ipc.onboardingChoice(true);
    await ipc.closeWindow();
  } catch (e) {
    const err = document.querySelector<HTMLElement>('#err')!;
    const manual = (await ipc.getSettings()).manual;
    document.querySelector<HTMLElement>('#keys')!.hidden = true; // the error lists the keys with their commands
    err.hidden = false;
    err.textContent = `Couldn't take over the shortcuts: ${String(e)}. Bind these yourself:`;
    for (const [k, c] of manual) {
      const code = document.createElement('code');
      code.textContent = `${k} → ${c}`;
      err.appendChild(code);
    }
    const yes = document.querySelector<HTMLButtonElement>('#yes')!;
    yes.textContent = 'Close';
    yes.onclick = () => void ipc.closeWindow();
  }
});
