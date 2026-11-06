import { describe, expect, it } from 'vitest';
import { shortcutOf, type ShortcutKey } from './shortcuts';

const none = { ctrlKey: false, metaKey: false, shiftKey: false, altKey: false };
const press = (key: string, modifiers: Partial<ShortcutKey> = {}): ShortcutKey => ({ key, ...none, ...modifiers });

describe('the operator\'s own shortcuts (RA-003)', () => {
  it.each([
    ['Ctrl+Shift+C', press('C', { ctrlKey: true, shiftKey: true }), 'copy'],
    ['Cmd+C', press('c', { metaKey: true }), 'copy'],
    ['Ctrl+Shift+V', press('V', { ctrlKey: true, shiftKey: true }), 'paste'],
    ['Cmd+V', press('v', { metaKey: true }), 'paste'],
    ['Cmd+F', press('f', { metaKey: true }), 'find'],
    ['Ctrl+Shift+F', press('F', { ctrlKey: true, shiftKey: true }), 'find'],
  ])('RA-003: %s is the terminal\'s own', (_name, event, action) => {
    expect(shortcutOf(event)).toBe(action);
  });

  it.each([
    ['Ctrl+C', press('c', { ctrlKey: true })],
    ['Ctrl+F', press('f', { ctrlKey: true })],
    ['Ctrl+V', press('v', { ctrlKey: true })],
    ['plain c', press('c')],
    ['Alt+Cmd+C', press('c', { metaKey: true, altKey: true })],
    ['Ctrl+Shift+Cmd+C', press('c', { ctrlKey: true, shiftKey: true, metaKey: true })],
    ['Shift+Cmd+F', press('f', { metaKey: true, shiftKey: true })],
    ['Ctrl+D', press('d', { ctrlKey: true })],
  ])('RA-003: %s stays with the shell', (_name, event) => {
    expect(shortcutOf(event)).toBeUndefined();
  });
});
