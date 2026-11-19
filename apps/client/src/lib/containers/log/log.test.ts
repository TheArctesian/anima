import { expect, it } from 'vitest';
import type { LogLine } from '../api';
import { downloadName, downloadText, levelOf, shownLines } from './log';

const line = (text: string, at: string | null = null): LogLine => ({ at, text });

const LINES: readonly LogLine[] = [
  line('INFO listening on :3000', '2026-11-18T18:00:00+00:00'),
  line('[warn] slow query 1200 ms', '2026-11-18T18:00:01+00:00'),
  line('{"level":"error","msg":"connection refused"}', '2026-11-18T18:00:02+00:00'),
  line('level=debug cache miss'),
  line('GET /health 200'),
];

it('HOST-015: the level of a line is its first level word, in any case and any form', () => {
  expect(LINES.map((each) => levelOf(each.text))).toEqual(['info', 'warning', 'error', 'debug', null]);
  expect(levelOf('FATAL: database is gone')).toBe('error');
  expect(levelOf('TRACE enter')).toBe('debug');
  // A word that only holds a level word is not one.
  expect(levelOf('information desk, terror, warned')).toBeNull();
  // A level word far into a long line is text of the message, not its level.
  expect(levelOf(`${'x'.repeat(130)} error`)).toBeNull();
});

it('HOST-015: the filter shows the lines of one level, and all shows each line', () => {
  expect(shownLines(LINES, { search: '', level: 'all' })).toHaveLength(5);
  expect(shownLines(LINES, { search: '', level: 'error' }).map((each) => each.text)).toEqual(['{"level":"error","msg":"connection refused"}']);
  expect(shownLines(LINES, { search: '', level: 'info' })).toHaveLength(1);
});

it('HOST-015: the search finds text in any case, together with the level', () => {
  expect(shownLines(LINES, { search: ' QUERY ', level: 'all' }).map((each) => each.text)).toEqual(['[warn] slow query 1200 ms']);
  expect(shownLines(LINES, { search: 'query', level: 'error' })).toEqual([]);
  // The search text is text, not a pattern.
  expect(shownLines(LINES, { search: '.*', level: 'all' })).toEqual([]);
});

it('HOST-015: a download is the shown lines, with the time of the hub when timestamps are on', () => {
  const shown = shownLines(LINES, { search: '', level: 'warning' });
  expect(downloadText(shown, true)).toBe('2026-11-18T18:00:01+00:00 [warn] slow query 1200 ms\n');
  expect(downloadText(shown, false)).toBe('[warn] slow query 1200 ms\n');
  expect(downloadText([line('no time')], true)).toBe('no time\n');
});

it('HOST-015: the name of the file has only safe characters of the name of the container', () => {
  expect(downloadName('tracks-app-1')).toBe('tracks-app-1.log');
  expect(downloadName('../../etc/passwd')).toBe('etc-passwd.log');
  expect(downloadName('a b\u0000c')).toBe('a-b-c.log');
  expect(downloadName('///')).toBe('container.log');
});
