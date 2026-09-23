type KeyLike = Pick<KeyboardEvent, 'key' | 'code' | 'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey'>;

/** KeyboardEvent → neutral "Ctrl+Alt+Shift+R" combo, or null if rshot can't take it.
 *  A letter or digit is the one the layout types (AZERTY's A key is "A", as GNOME and the Windows hook
 *  match it), else the US key at that spot (Shift+4 types "$", Cyrillic types no Latin letter).
 *  Same rule as combo::takeable: Ctrl, Alt or Super, or Print; anything else is taken from every app. */
export function comboFrom(e: KeyLike): string | null {
  const c = e.code;
  const typed = /^[a-z0-9]$/i.test(e.key) && !c.startsWith('Numpad') ? e.key.toUpperCase() : undefined;
  const key = c === 'PrintScreen' ? 'Print' : /^F\d{1,2}$/.test(c) ? c : (typed ?? /^(?:Key|Digit)(.)$/.exec(c)?.[1]);
  if (!key || !(e.ctrlKey || e.altKey || e.metaKey || key === 'Print')) return null;
  const mods = [e.ctrlKey && 'Ctrl', e.altKey && 'Alt', e.shiftKey && 'Shift', e.metaKey && 'Super'].filter(
    (m): m is string => !!m,
  );
  return [...mods, key].join('+');
}
