import { expect, it } from 'vitest';
import { elapsedText } from './elapsed';

it('shows minutes and seconds, and the hours only after an hour', () => {
  expect(elapsedText(0)).toBe('00:00');
  expect(elapsedText(252_900)).toBe('04:12');
  expect(elapsedText(3_852_000)).toBe('1:04:12');
});
