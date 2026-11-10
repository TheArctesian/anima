import { beforeEach, expect, it } from 'vitest';
import { asciicastStream } from '$lib/fixtures/phase3';
import { parseCast, type Cast } from './parse';
import type { PlaybackTerminal } from '../../screen/playback';
import { createCastPlayer, type PlaybackClock } from './player';

/** A terminal that keeps the screen text and the calls made to it. */
function recordingTerminal() {
  const calls: string[] = [];
  let screen = '';
  const size = { cols: 0, rows: 0 };
  const terminal: PlaybackTerminal = {
    write: (text) => {
      screen += text;
      calls.push('write');
    },
    resize: (cols, rows) => {
      size.cols = cols;
      size.rows = rows;
      calls.push(`resize ${String(cols)}x${String(rows)}`);
    },
    reset: () => {
      screen = '';
      calls.push('reset');
    },
  };
  return { terminal, calls, screen: () => screen, size };
}

/** A clock that moves only when the test says so. */
function fakeClock() {
  let now = 0;
  const timers: { due: number; run: () => void; live: boolean }[] = [];
  const clock: PlaybackClock = {
    now: () => now,
    after: (ms, run) => {
      const timer = { due: now + ms, run, live: true };
      timers.push(timer);
      return () => {
        timer.live = false;
      };
    },
  };
  const advance = (ms: number) => {
    const target = now + ms;
    for (;;) {
      const next = timers.filter((t) => t.live && t.due <= target).sort((a, b) => a.due - b.due)[0];
      if (next === undefined) break;
      next.live = false;
      now = Math.max(now, next.due);
      next.run();
    }
    now = target;
  };
  return { clock, advance, pending: () => timers.filter((t) => t.live).length };
}

const text = [
  '{"version":2,"width":80,"height":24,"timestamp":0}',
  '[1,"o","one "]',
  '[2,"o","two "]',
  '[2.5,"r","100x30"]',
  '[3,"o","three"]',
].join('\n');
let cast: Cast;
beforeEach(() => {
  const parsed = parseCast(text, true);
  if (!parsed.ok) throw new Error('fixture refused');
  cast = parsed.cast;
});

function setup(source: Cast = cast) {
  const t = recordingTerminal();
  const c = fakeClock();
  const player = createCastPlayer(t.terminal, source, c.clock);
  return { ...t, ...c, player };
}

it('RA-041: the terminal starts at the recorded size with nothing written and the player paused', () => {
  const { player, screen, size } = setup();
  expect(size).toEqual({ cols: 80, rows: 24 });
  expect(screen()).toBe('');
  expect(player.view()).toEqual({ playing: false, position: 0, duration: 3 });
});

it('D-12: play advances with the injected clock and writes each output at its time, as recorded', () => {
  const { player, advance, screen } = setup();
  player.play();
  advance(999);
  expect(screen()).toBe('');
  advance(1);
  expect(screen()).toBe('one ');
  advance(1000);
  expect(screen()).toBe('one two ');
  expect(player.view().position).toBe(2);
});

it('D-12: resize events resize the terminal at their time', () => {
  const { player, advance, size } = setup();
  player.play();
  advance(2400);
  expect(size).toEqual({ cols: 80, rows: 24 });
  advance(100);
  expect(size).toEqual({ cols: 100, rows: 30 });
});

it('D-12: pause holds the position and the screen, and play carries on from there', () => {
  const { player, advance, screen, pending } = setup();
  player.play();
  advance(1500);
  player.pause();
  expect(pending()).toBe(0);
  advance(5000);
  expect(screen()).toBe('one ');
  expect(player.view()).toEqual({ playing: false, position: 1.5, duration: 3 });
  player.play();
  advance(500);
  expect(screen()).toBe('one two ');
});

it('D-12: the end stops at the duration, announces, and play again starts from the top', () => {
  const { player, advance, screen, calls } = setup();
  let changes = 0;
  player.onchange(() => (changes += 1));
  player.play();
  advance(10_000);
  expect(player.view()).toEqual({ playing: false, position: 3, duration: 3 });
  expect(screen()).toBe('one two three');
  expect(changes).toBe(2);
  player.play();
  expect(calls.filter((c) => c === 'reset')).toHaveLength(1);
  expect(screen()).toBe('');
  advance(1000);
  expect(screen()).toBe('one ');
});

it('RA-041: seek(t) shows exactly the output up to t and the size at t, with one reset when it goes back', () => {
  const { player, advance, screen, calls, size } = setup();
  player.play();
  advance(3000);
  calls.length = 0;
  player.seek(2.2);
  expect(screen()).toBe('one two ');
  expect(size).toEqual({ cols: 80, rows: 24 });
  expect(calls.filter((c) => c === 'reset')).toHaveLength(1);
  expect(player.view().position).toBe(2.2);
});

it('RA-041: seek forward writes only the new output, without a reset', () => {
  const { player, screen, calls, size } = setup();
  player.seek(1);
  calls.length = 0;
  player.seek(2.7);
  expect(calls).not.toContain('reset');
  expect(screen()).toBe('one two ');
  expect(size).toEqual({ cols: 100, rows: 30 });
});

it('RA-041: seek replays the resize events in order, so the screen is the one the recording had at t', () => {
  const { player, calls } = setup();
  player.seek(3);
  player.seek(0.5);
  player.seek(3);
  const afterReset = calls.slice(calls.lastIndexOf('reset'));
  expect(afterReset).toEqual(['reset', 'resize 80x24', 'write', 'resize 100x30', 'write']);
});

it('D-12: seek keeps playing from the new time when it was playing, and holds when paused', () => {
  const { player, advance, screen } = setup();
  player.seek(2.2);
  advance(5000);
  expect(player.view().playing).toBe(false);
  expect(screen()).toBe('one two ');
  player.play();
  player.seek(0);
  expect(player.view().playing).toBe(true);
  advance(1000);
  expect(screen()).toBe('one ');
});

it('D-12: seek clamps to the recording, and seek to the end while playing ends it', () => {
  const { player, screen } = setup();
  player.play();
  player.seek(99);
  expect(player.view()).toEqual({ playing: false, position: 3, duration: 3 });
  expect(screen()).toBe('one two three');
  player.seek(-5);
  expect(player.view().position).toBe(0);
});

it('RA-041: a recording read further extends the player, which keeps its position and plays on', () => {
  const short = parseCast(text.split('\n').slice(0, 3).join('\n') + '\n', false);
  if (!short.ok) throw new Error('refused');
  const { player, advance, screen } = setup(short.cast);
  player.play();
  advance(1500);
  player.extend(cast);
  expect(player.view()).toEqual({ playing: true, position: 1.5, duration: 3 });
  advance(1500);
  expect(screen()).toBe('one two three');
});

it('RA-041: the K6 asciicast fixture plays to its last output and seeks back to a screen', () => {
  const parsed = parseCast(asciicastStream.text, true);
  if (!parsed.ok) throw new Error('refused');
  const { player, advance, screen } = setup(parsed.cast);
  player.play();
  advance(2000);
  expect(screen()).toBe('$ echo hello\r\nhello\r\n$ ');
  player.seek(0.9);
  expect(screen()).toBe('$ echo hello\r\n');
});
