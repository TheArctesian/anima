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

it('HOST-015: a field that names the level wins, and then the first level word counts', () => {
  expect(levelOf('{"msg":"error connecting","level":"info"}')).toBe('info');
  expect(levelOf('time=10:00 lvl=warn msg="ERROR in the text"')).toBe('warning');
  expect(levelOf('WARN retry after ERROR')).toBe('warning');
  expect(levelOf('ERROR after a [warn]')).toBe('error');
});

it('HOST-015: the forms that common programs write are read, also inside the colour codes of a terminal', () => {
  expect(levelOf('\u001b[31mERROR\u001b[0m boom')).toBe('error');
  expect(levelOf('ERRO[0001] failed to get info')).toBe('error');
  expect(levelOf('E1118 10:00:00.000000       1 reflector.go:1] failed')).toBe('error');
  expect(levelOf('W1118 10:00:00.000000       1 x.go:1] slow')).toBe('warning');
  expect(levelOf('10:00AM WRN slow')).toBe('warning');
  expect(levelOf('2026/11/18 10:00:00 [emerg] 1#1: bind() failed')).toBe('error');
  expect(levelOf('error: connection refused')).toBe('error');
});

it('HOST-015: a word in small letters in the text of a line is not its level', () => {
  for (const text of ['GET /debug/vars 200', 'GET /api/info 200', 'open /var/log/nginx/error.log', 'trace-id=ab12 served', 'no errors found']) expect(levelOf(text)).toBeNull();
  // A word that the limit of 120 characters would cut into a level word is not one.
  expect(levelOf(`${'x'.repeat(113)} ERRORS`)).toBeNull();
});

it('HOST-015: a line with no level has the level of the line before it, so a stack trace stays with its error', () => {
  const trace = [line('INFO started'), line('ERROR boom'), line('    at main (app.js:1)'), line('    at run (app.js:9)'), line('INFO again'), line('plain')];
  expect(shownLines(trace, { search: '', level: 'error' }).map((each) => each.text)).toEqual(['ERROR boom', '    at main (app.js:1)', '    at run (app.js:9)']);
  expect(shownLines(trace, { search: '', level: 'info' }).map((each) => each.text)).toEqual(['INFO started', 'INFO again', 'plain']);
  // A line before the first level word has no level.
  expect(shownLines([line('banner'), line('WARN x')], { search: '', level: 'warning' })).toHaveLength(1);
});

it('HOST-015: the filter shows the lines of one level, and all shows each line', () => {
  expect(shownLines(LINES, { search: '', level: 'all' })).toHaveLength(5);
  expect(shownLines(LINES, { search: '', level: 'error' }).map((each) => each.text)).toEqual(['{"level":"error","msg":"connection refused"}']);
  // `GET /health 200` states no level and follows the debug line.
  expect(shownLines(LINES, { search: '', level: 'debug' })).toHaveLength(2);
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
