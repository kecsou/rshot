type KeyLike = Pick<KeyboardEvent, 'code' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey'>;

/** KeyboardEvent → neutral "Ctrl+Alt+Shift+R" combo, or null if it can't be a shortcut. */
export function comboFrom(e: KeyLike): string | null {
  const c = e.code;
  const key =
    c === 'PrintScreen' ? 'Print' : c.startsWith('Key') ? c.slice(3) : c.startsWith('Digit') ? c.slice(5) : /^F\d{1,2}$/.test(c) ? c : null;
  if (!key) return null;
  const mods = [e.ctrlKey && 'Ctrl', e.altKey && 'Alt', e.shiftKey && 'Shift', e.metaKey && 'Super'].filter(
    (m): m is string => !!m,
  );
  if (!mods.length && key !== 'Print' && !/^F\d{1,2}$/.test(key)) return null;
  return [...mods, key].join('+');
}
