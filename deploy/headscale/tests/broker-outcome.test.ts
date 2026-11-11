// The broker's checks of what a change left in Headscale (vpn-plan.md Section 5.1, VPN-003).
import { describe, expect, it } from 'bun:test';
import { audit, fixStep, staggered, type Issued } from '../broker/outcome.ts';
import { NOW, inDays, listed, router, tailnetOfNodes } from './broker-fixture.ts';

const at = (hoursAgo: number): string => new Date(NOW.getTime() - hoursAgo * 3_600_000).toISOString();
const personal = (keyId: number, passed = false): Issued => ({ keyId, verb: 'personal-key', at: at(0.5), passed });
const routerKey = (keyId: number, site = 'home', passed = false): Issued => ({ keyId, verb: 'router-key', site, at: at(0.5), passed });

describe('audit', () => {
  it('VPN-003: a personal node with a tag is deleted', () => {
    const tailnet = tailnetOfNodes([listed(3, { tags: ['tag:hub'], user: 'tagged-devices', expiry: null, preAuthKeyId: 11 })]);
    const result = audit(tailnet, [personal(11)], null, NOW);
    expect(result.fixes).toEqual([{ kind: 'delete', node: 3, reason: 'the node had a tag' }]);
    expect(result.issued).toEqual([{ ...personal(11), passed: false }]);
  });

  it('VPN-003: the registered node with a tag is deleted', () => {
    const tailnet = tailnetOfNodes([listed(6, { tags: ['tag:router'], expiry: null })]);
    expect(audit(tailnet, [], 6, NOW).fixes).toEqual([{ kind: 'delete', node: 6, reason: 'the node had a tag' }]);
  });

  it('a personal node with no expiry gets one 180 days out', () => {
    const tailnet = tailnetOfNodes([listed(3, { expiry: null, preAuthKeyId: 11 })]);
    const result = audit(tailnet, [personal(11)], null, NOW);
    expect(result.fixes).toEqual([{ kind: 'set-expiry', node: 3, expiry: inDays(180) }]);
    expect(result.issued[0]?.passed).toBe(true);
  });

  it('a personal node 181 days out is set back; 180 days is left alone', () => {
    expect(audit(tailnetOfNodes([listed(3, { expiry: inDays(181) })]), [], 3, NOW).fixes).toEqual([{ kind: 'set-expiry', node: 3, expiry: inDays(180) }]);
    expect(audit(tailnetOfNodes([listed(3, { expiry: inDays(180) })]), [], 3, NOW).fixes).toEqual([]);
  });

  it('a second Mac gets 150 days, a third 120 (stagger)', () => {
    const first = listed(1, { expiry: inDays(180) });
    const second = listed(2, { expiry: null, preAuthKeyId: 12 });
    const third = listed(3, { expiry: null, preAuthKeyId: 13 });
    const result = audit(tailnetOfNodes([first, second, third]), [personal(12), personal(13)], null, NOW);
    expect(result.fixes).toEqual([
      { kind: 'set-expiry', node: 2, expiry: inDays(150) },
      { kind: 'set-expiry', node: 3, expiry: inDays(120) },
    ]);
  });

  it('the stagger ignores tagged nodes and other users, and falls back to 90 days', () => {
    expect(staggered(NOW, [router(1, 'home'), listed(2, { user: 'intruder', expiry: inDays(180) })])).toEqual(inDays(180));
    const busy = [150, 120, 90, 60, 180].map((days, index) => listed(index + 1, { expiry: inDays(days) }));
    expect(staggered(NOW, busy)).toEqual(inDays(90));
  });

});

describe('audit of router keys', () => {
  it('a router node with an extra tag is deleted', () => {
    const node = listed(4, { user: 'tagged-devices', tags: ['tag:router', 'tag:site-home', 'tag:hub'], expiry: null, preAuthKeyId: 21 });
    expect(audit(tailnetOfNodes([node]), [routerKey(21)], null, NOW).fixes).toEqual([{ kind: 'delete', node: 4, reason: 'the router had other tags' }]);
  });

  it('a router node of another site is deleted; one with its own tags passes', () => {
    expect(audit(tailnetOfNodes([router(4, 'office1', 21)]), [routerKey(21)], null, NOW).fixes).toHaveLength(1);
    const passed = audit(tailnetOfNodes([router(4, 'home', 21)]), [routerKey(21)], null, NOW);
    expect(passed).toEqual({ fixes: [], issued: [{ ...routerKey(21), passed: true }] });
  });

  it('a router node with an expiry is deleted', () => {
    const node = { ...router(4, 'home', 21), expiry: inDays(180) };
    expect(audit(tailnetOfNodes([node]), [routerKey(21)], null, NOW).fixes).toEqual([{ kind: 'delete', node: 4, reason: 'the router had a key expiry' }]);
  });

  it('a node that passed once is not checked again', () => {
    const changed = listed(4, { user: 'tagged-devices', tags: ['tag:router', 'tag:site-home', 'tag:agent'], expiry: null, preAuthKeyId: 21 });
    const result = audit(tailnetOfNodes([changed]), [routerKey(21, 'home', true)], null, NOW);
    expect(result).toEqual({ fixes: [], issued: [routerKey(21, 'home', true)] });
  });

  it('an issued key with no node is dropped after 2 hours', () => {
    const young = { ...personal(11), at: at(1.9) };
    const old = { ...personal(12), at: at(2.1) };
    expect(audit(tailnetOfNodes([]), [young, old], null, NOW).issued).toEqual([young]);
  });

  it('nodes from keys the broker did not issue are left alone', () => {
    const tailnet = tailnetOfNodes([listed(1, { tags: ['tag:hub'], expiry: inDays(400), preAuthKeyId: 1 }), router(2, 'office1', 2), listed(3, { expiry: null })]);
    expect(audit(tailnet, [personal(11), routerKey(21)], null, NOW).fixes).toEqual([]);
  });

  it('one node gets one fix; a delete wins', () => {
    const tailnet = tailnetOfNodes([listed(3, { tags: ['tag:hub'], expiry: null, preAuthKeyId: 11 })]);
    expect(audit(tailnet, [personal(11)], 3, NOW).fixes).toHaveLength(1);
  });
});

describe('fixStep', () => {
  it('R-055: a delete is forced and names the node by id', () => {
    expect(fixStep({ kind: 'delete', node: 4, reason: 'x' })).toEqual(['nodes', 'delete', '--identifier=4', '--force']);
  });

  it('the expiry value is never empty', () => {
    expect(fixStep({ kind: 'set-expiry', node: 3, expiry: new Date('2027-05-09T00:00:00.123Z') })).toEqual(['nodes', 'expire', '--identifier=3', '--expiry=2027-05-09T00:00:00Z']);
    expect(() => fixStep({ kind: 'set-expiry', node: 3, expiry: new Date(Number.NaN) })).toThrow(RangeError);
  });
});
