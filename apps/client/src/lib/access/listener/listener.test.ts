import { describe, expect, it, vi } from 'vitest';
import type { ListenerResult } from '$lib/desktop/hub-listener/shell';
import { parseHubAddress, type HubAddress } from '$lib/hub/hub';
import { ListenerRefused, listenerGate, listenerProblem, needsListenerCheck, verdictOf } from './listener';

const LOCAL = 'http://127.0.0.1:8443';

function address(text: string): HubAddress {
  const parsed = parseHubAddress(text);
  if (!parsed.ok) throw new Error('bad address');
  return parsed.value;
}

describe('needsListenerCheck', () => {
  it('F-10: the desktop app checks every plain http hub address before it sends to it', () => {
    expect(needsListenerCheck(LOCAL, true)).toBe(true);
    expect(needsListenerCheck('http://localhost:41000', true)).toBe(true);
    expect(needsListenerCheck('not a url', true)).toBe(true);
  });

  it('F-10: an https hub and the browser are not checked', () => {
    expect(needsListenerCheck('https://hub.example.org', true)).toBe(false);
    expect(needsListenerCheck(LOCAL, false)).toBe(false);
  });
});

describe('the verdict', () => {
  it('F-10: only "own" is safe; another program, none or no answer is refused', () => {
    expect(verdictOf({ ok: true, listener: 'own' })).toEqual({ safe: true });
    expect(verdictOf({ ok: true, listener: 'other' })).toEqual({ safe: false, listener: 'other' });
    expect(verdictOf({ ok: true, listener: 'none' })).toEqual({ safe: false, listener: 'none' });
    expect(verdictOf({ ok: false, problem: 'bad-origin' })).toEqual({ safe: false, listener: 'unknown' });
  });
});

describe('listenerGate', () => {
  function gate(answer: ListenerResult) {
    const check = vi.fn(() => Promise.resolve(answer));
    const token = vi.fn(() => Promise.resolve('tok-secret'));
    return { check, token, gate: listenerGate(check) };
  }

  it('F-10: the token is read only after the check says this user owns the port', async () => {
    const own = gate({ ok: true, listener: 'own' });
    await expect(own.gate.token(own.token)(address(LOCAL))).resolves.toBe('tok-secret');
    expect(own.check).toHaveBeenCalledWith(LOCAL);
    expect(own.check.mock.invocationCallOrder[0]).toBeLessThan(own.token.mock.invocationCallOrder[0] ?? 0);
  });

  it.each(['other', 'none'] as const)('F-10, LOG-007: with %s listening the token is never read and the reader rejects with fixed text', async (listener) => {
    const refused = gate({ ok: true, listener });
    const read = refused.gate.token(refused.token)(address(LOCAL));
    await expect(read).rejects.toBeInstanceOf(ListenerRefused);
    await expect(read).rejects.not.toThrow(/127\.0\.0\.1/);
    expect(refused.token).not.toHaveBeenCalled();
  });

  it('F-10: a check that throws is refused too', async () => {
    const check = vi.fn(() => Promise.reject(new Error('shell gone')));
    expect(await listenerGate(check).verdict(LOCAL)).toEqual({ safe: false, listener: 'unknown' });
  });

  it('F-10: each origin is asked once per page load, and again after forget', async () => {
    const own = gate({ ok: true, listener: 'own' });
    await own.gate.verdict(LOCAL);
    await own.gate.verdict(LOCAL);
    expect(own.check).toHaveBeenCalledOnce();
    own.gate.forget(LOCAL);
    await own.gate.verdict(LOCAL);
    expect(own.check).toHaveBeenCalledTimes(2);
  });
});

describe('listenerProblem', () => {
  const JOINED = 'http://127.0.0.1:41000';

  it('F-10: a safe verdict has no problem', () => {
    expect(listenerProblem({ safe: true }, LOCAL, null)).toBeNull();
  });

  it('F-10: another program, nothing and no answer each have their own fixed text that never names the address', () => {
    const texts = (['other', 'none', 'unknown'] as const).map((listener) => listenerProblem({ safe: false, listener }, LOCAL, null));
    expect(texts.map((text) => text?.title)).toEqual(["Another program holds the hub's port", 'Nothing answers at the hub address', 'The hub address could not be checked']);
    for (const text of texts) expect(JSON.stringify(text)).not.toContain('127.0.0.1');
  });

  it('F-2, F-10: on a joined device the link says why: its port is taken, it is starting or it ended', () => {
    const refused = { safe: false, listener: 'none' } as const;
    expect(listenerProblem(refused, JOINED, { joined: true, state: 'down', port: 41_000, reason: 'port-taken' })?.advice).toBe(
      "Another program holds the link's port 41000. Close it, then check again.",
    );
    expect(listenerProblem(refused, JOINED, { joined: true, state: 'starting', port: 41_000 })?.title).toBe('The link to the hub is starting');
    expect(listenerProblem(refused, JOINED, { joined: true, state: 'down', port: 41_000, reason: 'ended' })?.advice).toContain('The link ended.');
  });

  it("F-10: a link on another port, or no link, leaves the listener's own text", () => {
    const refused = { safe: false, listener: 'other' } as const;
    expect(listenerProblem(refused, JOINED, { joined: true, state: 'down', port: 42_000, reason: 'port-taken' })?.title).toBe("Another program holds the hub's port");
    expect(listenerProblem(refused, JOINED, { joined: false, state: 'stopped' })?.title).toBe("Another program holds the hub's port");
  });
});
