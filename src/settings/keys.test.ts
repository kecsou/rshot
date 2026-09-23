import { describe, expect, it } from 'vitest';
import { comboFrom } from './keys';

type Mods = Partial<Record<'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey', boolean>>;
const ev = (code: string, key: string, mods: Mods = {}) => ({
  code,
  key,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
});

describe('comboFrom', () => {
  it('builds modifier combos in a fixed order', () => {
    expect(comboFrom(ev('KeyR', 'R', { ctrlKey: true, altKey: true, shiftKey: true }))).toBe('Ctrl+Alt+Shift+R');
    expect(comboFrom(ev('Digit4', '$', { metaKey: true, shiftKey: true }))).toBe('Shift+Super+4');
  });
  it('takes Print alone or with Shift, and anything with Ctrl, Alt or Super', () => {
    expect(comboFrom(ev('PrintScreen', 'PrintScreen'))).toBe('Print');
    expect(comboFrom(ev('PrintScreen', 'PrintScreen', { shiftKey: true }))).toBe('Shift+Print');
    expect(comboFrom(ev('F9', 'F9', { ctrlKey: true }))).toBe('Ctrl+F9');
  });
  it('refuses keys every app would lose: bare, Shift-only, lone modifiers', () => {
    expect(comboFrom(ev('F9', 'F9'))).toBeNull();
    expect(comboFrom(ev('KeyF', 'f'))).toBeNull();
    expect(comboFrom(ev('KeyA', 'A', { shiftKey: true }))).toBeNull();
    expect(comboFrom(ev('ShiftLeft', 'Shift', { shiftKey: true }))).toBeNull();
    expect(comboFrom(ev('ControlLeft', 'Control', { ctrlKey: true }))).toBeNull();
    expect(comboFrom(ev('Numpad1', '1', { ctrlKey: true }))).toBeNull();
  });
  it('records letters as the layout labels them, digits by their key', () => {
    expect(comboFrom(ev('KeyQ', 'a', { ctrlKey: true, altKey: true }))).toBe('Ctrl+Alt+A'); // AZERTY A
    expect(comboFrom(ev('Semicolon', 'm', { ctrlKey: true }))).toBe('Ctrl+M'); // AZERTY M
    expect(comboFrom(ev('KeyY', 'z', { ctrlKey: true }))).toBe('Ctrl+Z'); // QWERTZ Z
    // No Latin letter or digit typed: the US key at that spot.
    expect(comboFrom(ev('Digit1', '&', { ctrlKey: true }))).toBe('Ctrl+1'); // AZERTY digit row
    expect(comboFrom(ev('KeyZ', 'я', { ctrlKey: true }))).toBe('Ctrl+Z'); // Cyrillic
    expect(comboFrom(ev('KeyA', 'å', { altKey: true }))).toBe('Alt+A'); // macOS Option
    // A digit typed elsewhere is not the key's: its own digit, or nothing for a numpad key.
    expect(comboFrom(ev('Digit3', '1', { ctrlKey: true, shiftKey: true }))).toBe('Ctrl+Shift+3');
    expect(comboFrom(ev('KeyQ', '1', { ctrlKey: true }))).toBe('Ctrl+Q');
  });
});
