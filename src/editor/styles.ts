import * as M from './model';

export type Tool = 'select' | 'crop' | 'arrow' | 'rect' | 'ellipse' | 'line' | 'pen' | 'highlight' | 'text' | 'counter' | 'redact';
type Section = 'color' | 'width' | 'ends' | 'shadow' | 'size' | 'textStyle' | 'mode' | 'strength';

export const SECTIONS: Record<Tool, Section[]> = {
  select: [],
  crop: [],
  arrow: ['color', 'width', 'ends', 'shadow'],
  rect: ['color', 'width'],
  ellipse: ['color', 'width'],
  line: ['color', 'width'],
  pen: ['color', 'width'],
  highlight: ['color', 'width'],
  text: ['color', 'size', 'textStyle'],
  counter: ['color'],
  redact: ['mode', 'strength'],
};

/** The style new shapes get; the popover edits it, the selected shape adopts it. */
export const style = {
  color: '#ff453a',
  highlight: '#ffd60a',
  width: 1 as M.Width,
  ends: 'one' as M.Ends,
  shadow: true,
  size: 'M' as M.TextSize,
  textStyle: 'filled' as M.TextStyle,
  mode: 'pixelate' as M.RedactMode,
  strength: 0.6,
};

export const currentColor = (t: Tool) => (t === 'highlight' ? style.highlight : style.color);

const pop = document.querySelector<HTMLElement>('#pop')!;
let forTool: Tool = 'arrow';
let listener: () => void = () => {};

export const onStyleChange = (fn: () => void) => {
  listener = fn;
};
export const popoverOpen = () => !pop.hidden;
export const closePopover = () => {
  pop.hidden = true;
};

const cap = (s: string) => s[0].toUpperCase() + s.slice(1);

function section(s: Section): string {
  switch (s) {
    case 'color':
      return `<div class="lbl">Color</div><div class="sws">${M.SWATCHES.map(
        (c) => `<button data-color="${c}" style="background:${c}" aria-label="Color ${c}"></button>`,
      ).join('')}</div>`;
    case 'width':
      return `<div class="lbl">Width</div><div class="seg">${[0, 1, 2]
        .map((w) => `<button data-width="${w}" aria-label="Width ${'SML'[w]}"><b style="height:${[2, 4, 7][w]}px"></b></button>`)
        .join('')}</div>`;
    case 'ends':
      return `<div class="lbl">Ends</div><div class="seg">${(['one', 'both', 'none'] as const)
        .map((e) => `<button data-ends="${e}" aria-label="Arrowheads: ${e}"><svg class="ic sm" aria-hidden="true"><use href="#e-${e === 'both' ? 'two' : e}"/></svg></button>`)
        .join('')}</div>`;
    case 'shadow':
      return `<label class="tog">Drop shadow<button class="switch" role="switch" data-shadow="1"></button></label>`;
    case 'size':
      return `<div class="lbl">Size</div><div class="seg">${['S', 'M', 'L', 'XL'].map((z) => `<button data-size="${z}">${z}</button>`).join('')}</div>`;
    case 'textStyle':
      return `<div class="lbl">Style</div><div class="seg">${['filled', 'plain', 'outline'].map((v) => `<button data-tstyle="${v}">${cap(v)}</button>`).join('')}</div>`;
    case 'mode':
      return `<div class="lbl">Mode</div><div class="seg">${['pixelate', 'blur', 'solid'].map((v) => `<button data-rmode="${v}">${cap(v)}</button>`).join('')}</div>`;
    case 'strength':
      return `<div class="lbl">Strength</div><input type="range" min="0" max="1" step="0.05" data-strength="1" aria-label="Strength" />
        <p class="note">Drag over any area. The pixels are replaced in the saved file, so what's underneath can't be recovered.</p>`;
  }
}

function sync() {
  const on = (sel: string, match: (el: HTMLElement) => boolean) =>
    pop.querySelectorAll<HTMLElement>(sel).forEach((el) => el.classList.toggle('on', match(el)));
  on('[data-color]', (el) => el.dataset.color === currentColor(forTool));
  on('[data-width]', (el) => Number(el.dataset.width) === style.width);
  on('[data-ends]', (el) => el.dataset.ends === style.ends);
  on('[data-shadow]', () => style.shadow);
  on('[data-size]', (el) => el.dataset.size === style.size);
  on('[data-tstyle]', (el) => el.dataset.tstyle === style.textStyle);
  on('[data-rmode]', (el) => el.dataset.rmode === style.mode);
  pop.querySelector('[data-shadow]')?.setAttribute('aria-checked', String(style.shadow));
  const range = pop.querySelector<HTMLInputElement>('[data-strength]');
  if (range) range.value = String(style.strength);
}

/** Opens the style popover for `tool` next to `anchor` (a rail button). */
export function openPopover(tool: Tool, anchor: HTMLElement) {
  const sections = SECTIONS[tool];
  if (!sections.length) return;
  forTool = tool;
  pop.innerHTML = sections.map(section).join('');
  sync();
  pop.hidden = false;
  const host = (pop.offsetParent as HTMLElement).getBoundingClientRect();
  const a = anchor.getBoundingClientRect();
  const top = Math.min(a.top - host.top - 10, host.height - pop.offsetHeight - 8);
  pop.style.top = `${Math.max(8, top)}px`;
}

pop.addEventListener('click', (e) => {
  const b = (e.target as Element).closest<HTMLElement>('button');
  if (!b) return;
  const d = b.dataset;
  if (d.color) {
    if (forTool === 'highlight') style.highlight = d.color;
    else style.color = d.color;
  } else if (d.width) style.width = Number(d.width) as M.Width;
  else if (d.ends) style.ends = d.ends as M.Ends;
  else if (d.shadow) style.shadow = !style.shadow;
  else if (d.size) style.size = d.size as M.TextSize;
  else if (d.tstyle) style.textStyle = d.tstyle as M.TextStyle;
  else if (d.rmode) style.mode = d.rmode as M.RedactMode;
  else return;
  sync();
  listener();
});

// `change` (not `input`) so dragging the slider makes one undo step, not dozens.
pop.addEventListener('change', (e) => {
  const t = e.target as HTMLInputElement;
  if (!t.dataset.strength) return;
  style.strength = Number(t.value);
  listener();
});
