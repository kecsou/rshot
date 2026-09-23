import * as ipc from '../shared/ipc';

type BoolKey = 'show_thumbnail' | 'remember_selection' | 'show_pointer';

const pop = document.querySelector<HTMLElement>('#pop')!;
const optionsButton = () => document.querySelector<HTMLElement>('#bar [data-act="options"]');
let opts: ipc.OverlayOptions | null = null;

pop.innerHTML = `
  <div><div class="lbl">Save to</div>
    <button class="field" data-act="folder"><svg class="ic" aria-hidden="true"><use href="#i-folder"/></svg><span class="grow"></span><svg class="ic sm" aria-hidden="true"><use href="#i-down"/></svg></button></div>
  <div><div class="lbl">Timer</div>
    <div class="seg">${[0, 3, 5, 10].map((n) => `<button data-timer="${n}">${n ? `${n} s` : 'Off'}</button>`).join('')}</div></div>
  <div class="full"></div>
  <div class="toggles">
    <div class="tog">Show floating thumbnail<button class="switch" role="switch" data-opt="show_thumbnail" aria-label="Show floating thumbnail"></button></div>
    <div class="tog">Remember last selection<button class="switch" role="switch" data-opt="remember_selection" aria-label="Remember last selection"></button></div>
    <div class="tog">Show mouse pointer<button class="switch" role="switch" data-opt="show_pointer" aria-label="Show mouse pointer" title="Applies from the next capture"></button></div>
  </div>
  <div><div class="lbl">Microphone (recording)</div><select class="field" id="mic" aria-label="Microphone"><option value="">None</option></select></div>`;

const micSel = pop.querySelector<HTMLSelectElement>('#mic')!;
micSel.addEventListener('change', async () => {
  if (!opts) return;
  opts.mic = micSel.value || null;
  await ipc.setOverlayOptions(opts);
});

/** Refills the mic list (a mic plugged in since shows up), keeping the configured one selected. */
function fillMics() {
  void ipc
    .listMics()
    .then((mics) => {
      micSel.length = 1; // keep "None"
      mics.forEach((m) => micSel.add(new Option(m.label, m.id)));
      const mic = opts?.mic;
      if (mic && !mics.some((m) => m.id === mic)) micSel.add(new Option(`${mic} (unavailable)`, mic));
      if (opts) renderOptions(opts);
    })
    .catch(() => {});
}

export function renderOptions(o: ipc.OverlayOptions) {
  opts = o;
  pop.querySelector('.grow')!.textContent = o.screenshots_dir;
  pop.querySelectorAll<HTMLElement>('[data-timer]').forEach((b) => b.classList.toggle('on', Number(b.dataset.timer) === o.timer_secs));
  pop.querySelectorAll<HTMLElement>('[data-opt]').forEach((b) => {
    const on = o[b.dataset.opt as BoolKey];
    b.classList.toggle('on', on);
    b.setAttribute('aria-checked', String(on));
  });
  micSel.value = o.mic ?? '';
}

/** "Show mouse pointer" works on Linux only: elsewhere its toggle goes. */
export function showPointerOption(supported: boolean) {
  pop.querySelector<HTMLElement>('[data-opt="show_pointer"]')!.closest<HTMLElement>('.tog')!.hidden = !supported;
}

export const optionsOpen = () => !pop.hidden;

export function closeOptions() {
  if (pop.contains(document.activeElement)) (document.activeElement as HTMLElement).blur();
  pop.hidden = true;
  optionsButton()?.classList.remove('on');
}

export function toggleOptions() {
  pop.hidden = !pop.hidden;
  if (!pop.hidden) fillMics();
  optionsButton()?.classList.toggle('on', !pop.hidden);
}

pop.addEventListener('click', async (e) => {
  // A toggle row's text flips its switch, as a label would (without taking focus).
  const t = e.target as Element;
  const b = t.closest<HTMLElement>('button') ?? t.closest('.tog')?.querySelector<HTMLElement>('button');
  if (!b || !opts) return;
  if (b.dataset.timer) opts.timer_secs = Number(b.dataset.timer);
  else if (b.dataset.opt) {
    const key = b.dataset.opt as BoolKey;
    opts[key] = !opts[key];
  } else if (b.dataset.act === 'folder') {
    const dir = await ipc.pickFolder();
    if (!dir) return;
    opts.screenshots_dir = dir;
  }
  renderOptions(opts);
  await ipc.setOverlayOptions(opts);
});
