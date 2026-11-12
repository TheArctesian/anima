import { describe, expect, it } from 'vitest';
import { arrow, backspace, codeOf, EMPTY_DIGITS, enter, isComplete } from './code-entry';

describe('NAV-009: the six code boxes', () => {
  it('typing a digit fills the box and moves focus to the next', () => {
    expect(enter(EMPTY_DIGITS, 0, '4')).toEqual({ digits: ['4', '', '', '', '', ''], focus: 1 });
  });

  it('typing in the last box keeps focus there', () => {
    expect(enter(['1', '2', '3', '4', '5', ''], 5, '6').focus).toBe(5);
  });

  it('typing over a filled box replaces its digit', () => {
    expect(enter(['1', '2', '', '', '', ''], 0, '9').digits.slice(0, 2)).toEqual(['9', '2']);
  });

  it('a letter or a symbol is ignored', () => {
    expect(enter(EMPTY_DIGITS, 2, 'a')).toEqual({ digits: EMPTY_DIGITS, focus: 2 });
    expect(enter(EMPTY_DIGITS, 2, '-')).toEqual({ digits: EMPTY_DIGITS, focus: 2 });
  });

  it('a pasted string with six digits fills every box, whichever box it went into', () => {
    expect(enter(EMPTY_DIGITS, 3, '123 456')).toEqual({ digits: ['1', '2', '3', '4', '5', '6'], focus: 5 });
    expect(codeOf(enter(EMPTY_DIGITS, 0, 'code: 987654.').digits)).toBe('987654');
  });

  it('more than six digits keep the first six', () => {
    expect(codeOf(enter(EMPTY_DIGITS, 0, '12345678').digits)).toBe('123456');
  });

  it('fewer than six pasted digits fill from the box and stop at the end', () => {
    expect(enter(EMPTY_DIGITS, 1, '78').digits).toEqual(['', '7', '8', '', '', '']);
    expect(enter(EMPTY_DIGITS, 4, '789').digits).toEqual(['', '', '', '', '7', '8']);
  });

  it('is complete only when all six boxes hold a digit', () => {
    expect(isComplete(['1', '2', '3', '4', '5', ''])).toBe(false);
    expect(isComplete(enter(['1', '2', '3', '4', '5', ''], 5, '6').digits)).toBe(true);
  });

  it('Backspace on an empty box moves to the previous box and clears it', () => {
    expect(backspace(['1', '2', '', '', '', ''], 2)).toEqual({ digits: ['1', '', '', '', '', ''], focus: 1 });
  });

  it('Backspace on a filled box clears it and stays', () => {
    expect(backspace(['1', '2', '', '', '', ''], 1)).toEqual({ digits: ['1', '', '', '', '', ''], focus: 1 });
  });

  it('Backspace on an empty first box does nothing', () => {
    expect(backspace(EMPTY_DIGITS, 0)).toEqual({ digits: EMPTY_DIGITS, focus: 0 });
  });

  it('the arrows move one box and stop at the ends', () => {
    expect(arrow(2, -1)).toBe(1);
    expect(arrow(2, 1)).toBe(3);
    expect(arrow(0, -1)).toBe(0);
    expect(arrow(5, 1)).toBe(5);
  });
});
