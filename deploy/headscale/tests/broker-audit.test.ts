// The broker's audit and limits end to end against a fake Headscale (vpn-plan.md Section 5.1):
// the audit timer (H2), unreadable nodes (L3), keys not yet used (M1), one router per site (M2)
// and the request deadline (H1).
import { afterEach, describe, expect, it } from 'bun:test';
import { changes, STEP_MS, withDeadline } from '../broker/deadline.ts';
import { serve, type Served } from '../broker/serve.ts';
import { auditTailnet } from '../broker/settle.ts';
import { readState, writeState, EMPTY_STATE, type BrokerFiles } from '../broker/state.ts';
import { LIST_STEPS, USERS_STEP, type Step } from '../broker/verbs.ts';
import { brokerFiles, FakeHeadscale, removeBrokerFiles } from './broker-fake.ts';

const NOW = new Date('2026-11-10T00:00:00Z');
const AUTH_ID = 'hskey-authreq-Ab3_-Zz9Ab3_-Zz9Ab3_-Zz9';
const SECONDS = Math.floor(NOW.getTime() / 1000);
afterEach(removeBrokerFiles);

const ask = (fake: FakeHeadscale, files: BrokerFiles, request: object): Promise<Served> => serve(JSON.stringify(request), fake.runner, files, fake.now);

function withHub(): FakeHeadscale {
  const fake = new FakeHeadscale(NOW);
  fake.add('hub', { tags: ['tag:hub'], expiry: null });
  return fake;
}

describe('the audit timer (H2)', () => {
  it('H2: a bad personal node is fixed while the hub is silent', async () => {
    const fake = withHub();
    const files = brokerFiles();
    expect((await ask(fake, files, { verb: 'personal-key' })).reply.ok).toBe(true);
    const pc = fake.join(1, 'pc', { expiry: SECONDS + 3650 * 86_400 });
    const log = await auditTailnet(fake.runner, files, NOW);
    expect(fake.node(pc.id)?.expiry).toBe(SECONDS + 180 * 86_400);
    expect(log).toBe('anima-broker verb=audit outcome=ok node=- fixes=1');
    expect(fake.ran('preauthkeys', 'create')).toHaveLength(1);
  });

  it('H2: the audit finishes a register request a kill left behind', async () => {
    const fake = withHub();
    const files = brokerFiles();
    fake.add('sneaky', { tags: ['tag:hub'], expiry: null });
    await writeState(files.state, { ...EMPTY_STATE, pendingRegister: { before: [1], node: null } });
    await auditTailnet(fake.runner, files, NOW);
    expect(fake.nodes.map((node) => node.name)).toEqual(['hub']);
    expect((await readState(files.state)).pendingRegister).toBeNull();
  });

  it('H2: the audit never touches a node the broker did not make', async () => {
    const fake = withHub();
    fake.add('operator-made', { tags: ['tag:hub'], expiry: SECONDS + 3650 * 86_400 });
    await auditTailnet(fake.runner, brokerFiles(), NOW);
    expect(fake.steps.filter((step) => changes(step))).toEqual([]);
  });
});

describe('unreadable nodes (L3)', () => {
  it('L3: one node in an unknown shape stops no verb and no audit', async () => {
    const fake = withHub();
    const files = brokerFiles();
    expect((await ask(fake, files, { verb: 'personal-key' })).reply.ok).toBe(true);
    fake.join(1, 'pc', { tags: ['tag:hub'], expiry: null });
    const odd = fake.add('odd', { expiry: -62_135_596_800 });
    const nodes = await ask(fake, files, { verb: 'nodes' });
    expect(nodes.reply).toMatchObject({ ok: true, value: { nodes: [{ name: 'hub' }] } });
    expect(nodes.log).toBe('anima-broker verb=nodes outcome=ok node=- fixes=1 unreadable=1');
    expect(fake.nodes.map((node) => node.name)).toEqual(['hub', 'odd']);
    expect((await ask(fake, files, { verb: 'renew', node: odd.id })).reply).toMatchObject({ ok: false, refusal: 'not-allowed' });
  });

  it('L3: an unreadable node counts against max_nodes', async () => {
    const fake = withHub();
    fake.add('mac');
    const files = brokerFiles(3);
    const odd = fake.add('odd', { expiry: -62_135_596_800 });
    expect((await ask(fake, files, { verb: 'personal-key' })).reply).toMatchObject({ ok: false, refusal: 'node-limit' });
    fake.nodes.splice(fake.nodes.indexOf(odd), 1);
    expect((await ask(fake, files, { verb: 'personal-key' })).reply.ok).toBe(true);
  });
});

describe('keys not yet used (M1)', () => {
  it('M1: keys that can still join count against max_nodes', async () => {
    const fake = withHub();
    fake.add('mac');
    const files = brokerFiles(3);
    expect((await ask(fake, files, { verb: 'personal-key' })).reply.ok).toBe(true);
    expect((await ask(fake, files, { verb: 'personal-key' })).reply).toMatchObject({ ok: false, refusal: 'node-limit' });
    fake.waiting.set(AUTH_ID, { name: 'laptop', tags: [] });
    expect((await ask(fake, files, { verb: 'register', authId: AUTH_ID })).reply).toMatchObject({ ok: false, refusal: 'node-limit' });
    fake.now = new Date(NOW.getTime() + 3_600_001);
    expect((await ask(fake, files, { verb: 'register', authId: AUTH_ID })).reply.ok).toBe(true);
    expect(fake.nodes).toHaveLength(3);
  });
});

describe('one router for each site (M2)', () => {
  it('M2: a second router for a site with an approved route is refused, its key too', async () => {
    const fake = withHub();
    const home = fake.add('router-home', { tags: ['tag:router', 'tag:site-home'], expiry: null, available: ['10.10.0.0/24'], approved: ['10.10.0.0/24'] });
    const rogue = fake.add('rogue', { tags: ['tag:router', 'tag:site-home'], expiry: null, available: ['10.10.0.0/24'] });
    const files = brokerFiles();
    expect((await ask(fake, files, { verb: 'approve-route', node: rogue.id, site: 'home' })).reply).toMatchObject({ ok: false, refusal: 'not-allowed' });
    expect(rogue.approved).toEqual([]);
    expect((await ask(fake, files, { verb: 'router-key', site: 'home' })).reply).toMatchObject({ ok: false, refusal: 'not-allowed' });
    expect(fake.keys).toEqual([]);
    home.approved = [];
    expect((await ask(fake, files, { verb: 'approve-route', node: rogue.id, site: 'home' })).reply.ok).toBe(true);
  });
});

describe('the request deadline (H1)', () => {
  const ran: Step[] = [];
  const runner = (step: Step): Promise<string> => {
    ran.push(step);
    return Promise.resolve('[]');
  };
  const DEADLINE = 45_000;
  const register: Step = ['auth', 'register', '--user=operator', `--auth-id=${AUTH_ID}`, '--output=json'];

  it('H1: a change is refused, not started, when it could run past the deadline', async () => {
    ran.length = 0;
    const guarded = withDeadline(runner, DEADLINE, () => DEADLINE - STEP_MS + 1);
    let refusal: unknown = null;
    await guarded(register).catch((error: unknown) => {
      refusal = error;
    });
    expect(refusal).toBeInstanceOf(Error);
    expect(String(refusal)).not.toContain(AUTH_ID);
    expect(ran).toEqual([]);
  });

  it('H1: reading steps still run near the deadline; a change in time runs', async () => {
    ran.length = 0;
    const late = withDeadline(runner, DEADLINE, () => DEADLINE - 1);
    for (const step of [...LIST_STEPS, USERS_STEP]) await late(step);
    await withDeadline(runner, DEADLINE, () => DEADLINE - STEP_MS)(register);
    expect(ran).toHaveLength(4);
  });
});
