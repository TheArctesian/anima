import { describe, expect, test } from 'bun:test';
import { staleFiles } from '../src/currency.ts';

const built = { 'a.css': 'one', 'b.ts': 'two' };

describe('generated files current (TOK-002)', () => {
  test('TOK-002 passes when every file equals its build', () => {
    expect(staleFiles(built, { ...built })).toEqual([]);
  });

  test('TOK-002 names a file whose text changed by one character', () => {
    expect(staleFiles(built, { 'a.css': 'one', 'b.ts': 'twO' })).toEqual([{ path: 'b.ts', reason: 'stale' }]);
  });

  test('TOK-002 names a file that is missing', () => {
    expect(staleFiles(built, { 'a.css': 'one', 'b.ts': undefined })).toEqual([{ path: 'b.ts', reason: 'missing' }]);
  });
});
