import { describe, expect, it, vi } from 'vitest';
import type { LocalHubResult, LocalHubStatus } from '$lib/desktop/local-hub/shell';
import { hubRoot, linkText, pollUntilSettled, POLL_MS, progressOf, stateText } from './state';

const READY: LocalHubStatus = { installed: true, running: true, state: 'ready', hub: '127.0.0.1:8443', link: { state: 'up', since: '2026-11-15T10:00:00Z' } };

describe('the state of the hub on this computer', () => {
  it('FRN-002: only ready, blocked and failed end the wait', () => {
    expect(progressOf(READY)).toBe('ready');
    expect(progressOf({ installed: true, running: true, state: 'blocked', reason: 'key-missing' })).toBe('blocked');
    expect(progressOf({ installed: true, running: false, state: 'failed', cause: 'anima-hub' })).toBe('failed');
    for (const state of ['starting', 'stopping', 'stopped', undefined] as const) expect(progressOf({ installed: true, running: true, ...(state === undefined ? {} : { state }) })).toBe('waiting');
  });

  it('FRN-006, LHB-010: each state has its own true sentence', () => {
    const states: LocalHubStatus[] = [
      { installed: false, running: false },
      { installed: true, running: true, state: 'starting' },
      READY,
      { ...READY, recovered: true },
      { installed: true, running: true, state: 'blocked', reason: 'database-version' },
      { installed: true, running: true, state: 'blocked', reason: 'database-in-use' },
      { installed: true, running: false, state: 'stopping' },
      { installed: true, running: false, state: 'stopped' },
      { installed: true, running: false, state: 'failed', cause: 'anima-gateway' },
    ];
    const texts = states.map((status) => stateText(status, true));
    expect(new Set(texts).size).toBe(texts.length);
    expect(texts[1]).toContain('can take a few minutes');
    expect(texts.at(-1)).toBe('Failed: anima-gateway ended. The hub starts again in 10 seconds.');
  });

  it('2.8, OR-S6-1: a missing key says to put vault.key back and where, and never offers a new key', () => {
    const blocked: LocalHubStatus = { installed: true, running: true, state: 'blocked', reason: 'key-missing' };
    expect(stateText(blocked, true)).toContain(`Put your copy of vault.key back in ${hubRoot(true)}`);
    expect(stateText(blocked, false)).toContain('~/.local/share/anima/hub');
    expect(stateText(blocked, true)).not.toMatch(/new key/i);
  });

  it('F-6: the link member has a sentence for each state, and none when absent', () => {
    expect(linkText(READY)).toBe('The link is up: devices can join.');
    expect(linkText({ ...READY, link: { state: 'down', since: '' } })).toContain('This computer still reaches the hub.');
    expect(linkText({ installed: true, running: true })).toBeNull();
  });
});

describe('pollUntilSettled', () => {
  function answers(...statuses: LocalHubResult<LocalHubStatus>[]) {
    const read = vi.fn<() => Promise<LocalHubResult<LocalHubStatus>>>();
    for (const status of statuses) read.mockResolvedValueOnce(status);
    return read;
  }
  const starting: LocalHubResult<LocalHubStatus> = { ok: true, value: { installed: true, running: true, state: 'starting', link: { state: 'starting', since: '' } } };

  it('FRN-002: reads once a second, shows each status and stops at ready', async () => {
    const read = answers(starting, starting, { ok: true, value: READY });
    const wait = vi.fn(() => Promise.resolve());
    const seen = vi.fn();
    const result = await pollUntilSettled({ read, wait, seen, stopped: () => false });
    expect(result).toEqual({ ok: true, value: READY });
    expect(read).toHaveBeenCalledTimes(3);
    expect(wait.mock.calls).toEqual([[POLL_MS], [POLL_MS]]);
    expect(seen).toHaveBeenCalledTimes(3);
  });

  it('FRN-002, LHB-010: a failed hub starts again by itself, so the poll goes on through failed and stops at blocked', async () => {
    const failed: LocalHubResult<LocalHubStatus> = { ok: true, value: { installed: true, running: false, state: 'failed', cause: 'anima-hub' } };
    const blocked: LocalHubResult<LocalHubStatus> = { ok: true, value: { installed: true, running: false, state: 'blocked', reason: 'key-missing' } };
    const seen = vi.fn();
    const result = await pollUntilSettled({ read: answers(failed, starting, blocked), wait: () => Promise.resolve(), seen, stopped: () => false });
    expect(result).toEqual(blocked);
    expect(seen).toHaveBeenCalledTimes(3);
  });

  it('FRN-002: a refusal of the shell ends the poll with it', async () => {
    const refused = { ok: false, problem: 'failed', message: 'x' } as const;
    expect(await pollUntilSettled({ read: answers(starting, refused), wait: () => Promise.resolve(), seen: vi.fn(), stopped: () => false })).toEqual(refused);
  });

  it('a screen that has gone stops the poll and drops the late answer', async () => {
    let gone = false;
    const seen = vi.fn();
    const read = vi.fn(() => {
      gone = true;
      return Promise.resolve(starting);
    });
    expect(await pollUntilSettled({ read, wait: () => Promise.resolve(), seen, stopped: () => gone })).toBeNull();
    expect(seen).not.toHaveBeenCalled();
  });
});
