// Each broker request as Headscale argv (R-055, BD-9, VP-7, VP-10). The flag names and forms are
// the ones Headscale 0.29.7 accepts (contract harness and the Build notes in vpn-build.md).
import { describe, expect, it } from 'bun:test';
import { nodesReply } from '../broker/nodes.ts';
import type { Request } from '../broker/protocol.ts';
import { plan, policyView, type Moment, type Step } from '../broker/verbs.ts';
import { parsePolicy } from '../src/policy.ts';
import { DAY_MS, NOW, inDays, listed, router, tailnetOfNodes, view } from './broker-fixture.ts';
import { raw, SITES } from './policy-fixture.ts';

const moment: Moment = { now: NOW, operatorId: 1 };
const home = router(2, 'home');
const routes = [
  { id: 2, approved: [], available: ['10.10.0.0/24'] },
  { id: 3, approved: [], available: ['10.20.0.0/24'] },
  { id: 4, approved: [], available: ['10.10.0.0/24', '10.20.0.0/24'] },
];
const tailnet = tailnetOfNodes([listed(1), home, router(3, 'office1'), router(4, 'home'), router(5, 'home'), listed(6, { expiry: inDays(170) })], routes);
const nodes = nodesReply(tailnet, view).nodes;
const steps = (request: Request, at: Moment = moment): ReturnType<typeof plan> => plan(request, view, nodes, at);
const requests: Request[] = [
  { verb: 'nodes' },
  { verb: 'personal-key' },
  { verb: 'router-key', site: 'home' },
  { verb: 'approve-route', node: 2, site: 'home' },
  { verb: 'register', authId: 'hskey-authreq-AAAAAAAAAAAAAAAAAAAAAAAA' },
  { verb: 'renew', node: 6 },
];
const allSteps = (): Step[] => requests.flatMap((request) => {
  const planned = steps(request);
  return typeof planned === 'string' ? [] : [...planned];
});

describe('policyView', () => {
  it('VPN-001: the sites are the policy hosts site-<x> with their subnets', () => {
    expect(Object.fromEntries(view.sites)).toEqual(SITES);
    expect(view.tags.has('tag:site-home')).toBe(true);
  });
});

describe('plan', () => {
  it('R-055: nodes reads the node and route lists', () => {
    expect(steps({ verb: 'nodes' })).toEqual([['nodes', 'list', '--output=json'], ['nodes', 'list-routes', '--output=json']]);
  });

  it('VPN-003: personal-key makes a one-hour key for the operator user id (a name is refused, fact 12)', () => {
    expect(steps({ verb: 'personal-key' })).toEqual([['preauthkeys', 'create', '--user=1', '--expiration=1h', '--output=json']]);
    expect(steps({ verb: 'personal-key' }, { now: NOW, operatorId: null })).toBe('headscale-failed');
  });

  it('VPN-001: router-key makes a one-hour key with the router and site tags', () => {
    expect(steps({ verb: 'router-key', site: 'home' })).toEqual([['preauthkeys', 'create', '--tags=tag:router,tag:site-home', '--expiration=1h', '--output=json']]);
    expect(steps({ verb: 'router-key', site: 'mars' })).toBe('unknown-site');
  });

  it('BD-9: a site with no tag in the policy is refused before Headscale', () => {
    const hosts = { ...(raw.hosts as Record<string, string>), 'site-lab': '10.40.0.0/24' };
    const withLab = policyView(parsePolicy({ ...raw, hosts }));
    expect(withLab.sites.get('lab')).toBe('10.40.0.0/24');
    expect(plan({ verb: 'router-key', site: 'lab' }, withLab, nodes, moment)).toBe('unknown-site');
  });

  it('R-055: approve-route approves the one waiting route of a router of the site', () => {
    expect(steps({ verb: 'approve-route', node: 2, site: 'home' })).toEqual([['nodes', 'approve-routes', '--identifier=2', '--routes=10.10.0.0/24']]);
  });

  it.each([
    ['a node of another site', 3, 'home', 'not-allowed'],
    ['the right tags but another site subnet', 3, 'office1', 'ok'],
    ['a node with two routes', 4, 'home', 'not-allowed'],
    ['a node with no route', 5, 'home', 'not-allowed'],
    ['an untagged node', 1, 'home', 'not-allowed'],
    ['an absent node', 99, 'home', 'not-allowed'],
    ['an unknown site', 2, 'mars', 'unknown-site'],
  ])('R-055: approve-route refuses %s', (_name, node, site, outcome) => {
    const planned = steps({ verb: 'approve-route', node, site });
    expect(typeof planned === 'string' ? planned : 'ok').toBe(outcome);
  });

});

describe('plan for register and renew', () => {
  it('VPN-003: register names the operator user and the auth ID', () => {
    expect(steps({ verb: 'register', authId: 'hskey-authreq-AAAAAAAAAAAAAAAAAAAAAAAA' })).toEqual([
      ['auth', 'register', '--user=operator', '--auth-id=hskey-authreq-AAAAAAAAAAAAAAAAAAAAAAAA', '--output=json'],
    ]);
  });

  it('VP-10: renew sets a staggered expiry on an untagged operator node', () => {
    // node1 expires in 170 days; 180 and 150 are within 30 days of it, so 120.
    expect(steps({ verb: 'renew', node: 6 })).toEqual([['nodes', 'expire', '--identifier=6', `--expiry=${inDays(120).toISOString().replace('.000Z', 'Z')}`]]);
  });

  it('VP-10: renew refuses a tagged node', () => {
    expect(steps({ verb: 'renew', node: 2 })).toBe('not-allowed');
    expect(steps({ verb: 'renew', node: 99 })).toBe('not-allowed');
    const foreign = nodesReply(tailnetOfNodes([listed(7, { user: 'intruder' })]), view).nodes;
    expect(plan({ verb: 'renew', node: 7 }, view, foreign, moment)).toBe('not-allowed');
  });

  it('VP-10: renew staggers against the other Macs', () => {
    const macs = nodesReply(tailnetOfNodes([listed(1, { expiry: inDays(180) }), listed(2, { expiry: inDays(150) }), listed(3, { expiry: inDays(10) })]), view).nodes;
    const planned = plan({ verb: 'renew', node: 3 }, view, macs, moment);
    expect(planned).toEqual([['nodes', 'expire', '--identifier=3', `--expiry=${inDays(120).toISOString().replace('.000Z', 'Z')}`]]);
  });

  it('VP-10: the expiry value is never empty', () => {
    for (let offset = 0; offset < 400; offset += 7) {
      const at = new Date(NOW.getTime() + offset * DAY_MS);
      const planned = steps({ verb: 'renew', node: 6 }, { now: at, operatorId: 1 });
      if (typeof planned === 'string') throw new Error(planned);
      const expiry = planned[0]?.find((arg) => arg.startsWith('--expiry='));
      expect(expiry).toMatch(/^--expiry=\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ$/);
    }
  });

  it('no step holds --reusable or --ephemeral', () => {
    expect(allSteps()).toHaveLength(7);
    for (const step of allSteps()) expect(step.filter((arg) => /reusable|ephemeral/.test(arg))).toEqual([]);
  });

  it('every value is in the same argv element as its flag', () => {
    for (const step of allSteps()) {
      for (const [index, arg] of step.entries()) {
        if (arg.startsWith('-')) expect([arg, /^--[a-z-]+=\S+$/.test(arg) || arg === '--force']).toEqual([arg, true]);
        else expect([arg, step[index - 1]?.startsWith('-') ?? false]).toEqual([arg, false]);
      }
    }
  });
});
