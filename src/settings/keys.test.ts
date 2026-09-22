import { describe, expect, it } from 'vitest';
import { comboFrom } from './keys';

const ev = (code: string, mods: Partial<Record<'ctrlKey' | 'altKey' | 'shiftKey' | 'metaKey', boolean>> = {}) => ({
  code,
  ctrlKey: false,
  altKey: false,
  shiftKey: false,
  metaKey: false,
  ...mods,
});

describe('comboFrom', () => {
  it('builds modifier combos in a fixed order', () => {
    expect(comboFrom(ev('KeyR', { ctrlKey: true, altKey: true, shiftKey: true }))).toBe('Ctrl+Alt+Shift+R');
    expect(comboFrom(ev('Digit4', { metaKey: true, shiftKey: true }))).toBe('Shift+Super+4');
  });
  it('accepts Print and F-keys alone', () => {
    expect(comboFrom(ev('PrintScreen'))).toBe('Print');
    expect(comboFrom(ev('F12'))).toBe('F12');
  });
  it('rejects bare letters and lone modifiers', () => {
    expect(comboFrom(ev('KeyF'))).toBeNull();
    expect(comboFrom(ev('ShiftLeft', { shiftKey: true }))).toBeNull();
  });
});
