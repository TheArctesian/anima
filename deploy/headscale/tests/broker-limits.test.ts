// The broker's rate and size limits (vpn-plan.md Section 5.1), with time as a parameter.
import { describe, expect, it } from 'bun:test';
import { admit, record, type LimitConfig, type LimitState } from '../broker/limits.ts';
import type { Verb } from '../broker/protocol.ts';
import { NOW } from './broker-fixture.ts';

const config: LimitConfig = { perDay: 3, maxNodes: 10 };
const empty: LimitState = { keys: [], registrations: [] };
const later = (seconds: number): Date => new Date(NOW.getTime() + seconds * 1000);
const after = (verbs: readonly Verb[], from: LimitState = empty): LimitState => verbs.reduce((state, verb, index) => record(verb, state, later(index)), from);

describe('admit and record', () => {
  it('5.1: the 4th key in 24 h is refused, also mixing personal and router keys', () => {
    const state = after(['personal-key', 'router-key', 'personal-key']);
    expect(state.keys).toHaveLength(3);
    expect(admit('router-key', { state, nodeCount: 1 }, config, later(60))).toBe('rate-limited');
    expect(admit('personal-key', { state, nodeCount: 1 }, config, later(60))).toBe('rate-limited');
    expect(admit('register', { state, nodeCount: 1 }, config, later(60))).toBe('ok');
  });

  it('5.1: the 4th registration in 24 h is refused', () => {
    const state = after(['register', 'register', 'register']);
    expect(admit('register', { state, nodeCount: 1 }, config, later(60))).toBe('rate-limited');
    expect(admit('personal-key', { state, nodeCount: 1 }, config, later(60))).toBe('ok');
  });

  it('5.1: a key 24 h + 1 s old no longer counts', () => {
    const state = after(['router-key', 'router-key', 'router-key']);
    expect(admit('router-key', { state, nodeCount: 1 }, config, later(86_400))).toBe('rate-limited');
    expect(admit('router-key', { state, nodeCount: 1 }, config, later(86_400 + 1))).toBe('ok');
    expect(record('nodes', state, later(86_400 + 3)).keys).toEqual([]);
  });

  it('5.1: node-limit refuses at the limit', () => {
    for (const verb of ['personal-key', 'router-key', 'register'] as const) {
      expect(admit(verb, { state: empty, nodeCount: 9 }, config, NOW)).toBe('ok');
      expect(admit(verb, { state: empty, nodeCount: 10 }, config, NOW)).toBe('node-limit');
    }
  });

  it('nodes and approve-route are never limited (nor renew, VP-10)', () => {
    const full = after(['personal-key', 'personal-key', 'personal-key', 'register', 'register', 'register']);
    for (const verb of ['nodes', 'approve-route', 'renew'] as const) {
      expect(admit(verb, { state: full, nodeCount: 99 }, config, NOW)).toBe('ok');
      expect(record(verb, full, NOW)).toEqual(full);
    }
  });

  it('5.1: an unreadable time still counts', () => {
    const state: LimitState = { keys: ['garbage', 'garbage', 'garbage'], registrations: [] };
    expect(admit('personal-key', { state, nodeCount: 0 }, config, NOW)).toBe('rate-limited');
  });
});
