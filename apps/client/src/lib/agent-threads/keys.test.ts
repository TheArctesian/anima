import { describe, expect, it } from 'vitest';
import { keyAction, moved, type Pressed } from './keys';

const press = (key: string, more: Partial<Pressed> = {}): Pressed => ({ key, mod: false, shift: false, alt: false, typing: false, ...more });

describe('the keys of the page Threads', () => {
  it('THR-016: the arrow keys move in the list, and Cmd+Shift+[ and ] open the previous and the next thread', () => {
    expect(keyAction(press('ArrowDown'))).toEqual({ kind: 'move', by: 1 });
    expect(keyAction(press('ArrowUp'))).toEqual({ kind: 'move', by: -1 });
    expect(keyAction(press('{', { mod: true, shift: true }))).toEqual({ kind: 'move', by: -1 });
    expect(keyAction(press(']', { mod: true, shift: true }))).toEqual({ kind: 'move', by: 1 });
  });

  it('THR-016: Cmd+1 to Cmd+9 open the thread at that place, / searches, E opens the tool calls', () => {
    expect(keyAction(press('1', { mod: true }))).toEqual({ kind: 'jump', place: 0 });
    expect(keyAction(press('9', { mod: true }))).toEqual({ kind: 'jump', place: 8 });
    expect(keyAction(press('0', { mod: true }))).toBeNull();
    expect(keyAction(press('/'))).toEqual({ kind: 'search' });
    expect(keyAction(press('e'))).toEqual({ kind: 'tools' });
  });

  it('THR-016: Cmd+K is not a key of this page, and a key with Alt is not', () => {
    expect(keyAction(press('k', { mod: true }))).toBeNull();
    expect(keyAction(press('ArrowDown', { alt: true }))).toBeNull();
  });

  it('THR-016: in a text field a letter is text; only Escape and the keys with Cmd act', () => {
    expect(keyAction(press('e', { typing: true }))).toBeNull();
    expect(keyAction(press('/', { typing: true }))).toBeNull();
    expect(keyAction(press('ArrowDown', { typing: true }))).toBeNull();
    expect(keyAction(press('Escape', { typing: true }))).toEqual({ kind: 'leave' });
    expect(keyAction(press('2', { mod: true, typing: true }))).toEqual({ kind: 'jump', place: 1 });
  });

  it('THR-016: a move stops at the ends of the list and starts at the near end', () => {
    expect(moved(-1, 1, 3)).toBe(0);
    expect(moved(-1, -1, 3)).toBe(2);
    expect(moved(2, 1, 3)).toBe(2);
    expect(moved(0, -1, 3)).toBe(0);
    expect(moved(0, 1, 0)).toBe(-1);
  });
});
