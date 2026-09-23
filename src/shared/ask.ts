// Prompts in the page's #modal (#modal-text, #modal-acts), which covers the whole window, header
// included: nothing else is clickable while one is up.
const $ = (s: string) => document.querySelector(s) as HTMLElement;

// A prompt holds the keyboard: Tab cycles its buttons, Esc picks the safe one, Enter the focused one.
addEventListener(
  'keydown',
  (e) => {
    if ($('#modal').hidden) return;
    e.stopPropagation();
    const bs = [...$('#modal-acts').children] as HTMLElement[];
    if (e.key === 'Escape') {
      e.preventDefault();
      (bs.find((b) => b.classList.contains('b2') && !b.classList.contains('danger')) ?? bs[0]).click();
    } else if (e.key === 'Tab') {
      e.preventDefault();
      const i = bs.indexOf(document.activeElement as HTMLElement);
      bs[i < 0 ? 0 : (i + (e.shiftKey ? bs.length - 1 : 1)) % bs.length].focus();
    }
  },
  true,
);

export function ask(text: string, buttons: { label: string; value: string; primary?: boolean; danger?: boolean }[]): Promise<string> {
  return new Promise((resolve) => {
    $('#modal-text').textContent = text;
    const acts = $('#modal-acts');
    acts.replaceChildren(
      ...buttons.map((b) => {
        const el = document.createElement('button');
        el.className = b.primary ? 'b1' : 'b2';
        if (b.danger) el.classList.add('danger');
        el.textContent = b.label;
        el.onclick = () => {
          $('#modal').hidden = true;
          resolve(b.value);
        };
        return el;
      }),
    );
    $('#modal').hidden = false;
    // Enter picks the safe choice: Cancel on a destructive prompt, else the primary action.
    (acts.querySelector<HTMLElement>('.b1:not(.danger)') ?? (acts.firstElementChild as HTMLElement | null))?.focus();
  });
}

export const fail = (what: string, e: unknown) => ask(`${what}: ${e}`, [{ label: 'OK', value: 'ok', primary: true }]);
