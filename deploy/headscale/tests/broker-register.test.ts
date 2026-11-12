// The register verb when its step is cut off or names a node the broker may not take
// (vpn-plan.md Section 5.1, VP-7, H1, L2), against a fake Headscale.
import { afterEach, describe, expect, it } from 'bun:test';
import { readFileSync } from 'node:fs';
import { serve, type Served } from '../broker/serve.ts';
import { readState, type BrokerFiles } from '../broker/state.ts';
import { brokerFiles, FakeHeadscale, removeBrokerFiles } from './broker-fake.ts';

const NOW = new Date('2026-11-10T00:00:00Z');
const AUTH_ID = 'hskey-authreq-Ab3_-Zz9Ab3_-Zz9Ab3_-Zz9';
afterEach(removeBrokerFiles);

const ask = (fake: FakeHeadscale, files: BrokerFiles, request: object): Promise<Served> => serve(JSON.stringify(request), fake.runner, files, fake.now);

function world(): { fake: FakeHeadscale; files: BrokerFiles } {
  const fake = new FakeHeadscale(NOW);
  fake.add('hub', { tags: ['tag:hub'], expiry: null });
  fake.add('mac');
  return { fake, files: brokerFiles() };
}

/** `auth register` runs in the container, then the docker CLI is killed (8 s) before it answers. */
function cutOff(fake: FakeHeadscale): void {
  fake.afterStep = (step) => {
    if (step[0] === 'auth') throw new Error('killed');
  };
}

describe('register: a step cut off after Headscale registered the node (H1)', () => {
  it('H1: the node it made is audited at once and the reply is a refusal', async () => {
    const { fake, files } = world();
    fake.waiting.set(AUTH_ID, { name: 'sneaky', tags: ['tag:hub'] });
    cutOff(fake);
    const served = await ask(fake, files, { verb: 'register', authId: AUTH_ID });
    expect(served.reply).toMatchObject({ ok: false, refusal: 'headscale-failed' });
    expect(fake.nodes.map((node) => node.name)).toEqual(['hub', 'mac']);
    expect(served.log).toBe('anima-broker verb=register outcome=headscale-failed node=- fixes=1');
    expect((await readState(files.state)).pendingRegister).toBeNull();
  });

  it('H1: an untagged node with a long expiry gets its expiry set back', async () => {
    const { fake, files } = world();
    fake.waiting.set(AUTH_ID, { name: 'laptop', tags: [] });
    fake.afterStep = (step) => {
      if (step[0] !== 'auth') return;
      const laptop = fake.nodes.find((node) => node.name === 'laptop');
      if (laptop !== undefined) laptop.expiry = Math.floor(NOW.getTime() / 1000) + 3650 * 86_400;
      throw new Error('killed');
    };
    await ask(fake, files, { verb: 'register', authId: AUTH_ID });
    expect(fake.nodes.find((node) => node.name === 'laptop')?.expiry).toBeLessThanOrEqual(Math.floor(NOW.getTime() / 1000) + 180 * 86_400);
  });

  it('H1: the node ids are saved before the step, so a killed process leaves them for the next run', async () => {
    const { fake, files } = world();
    fake.waiting.set(AUTH_ID, { name: 'sneaky', tags: ['tag:hub'] });
    let saved: unknown = null;
    fake.afterStep = (step) => {
      if (step[0] !== 'auth') return;
      saved = JSON.parse(readFileSync(files.state, 'utf8'));
      // The process dies here: this step and every later one (the audit) fail.
      fake.afterStep = () => {
        throw new Error('killed');
      };
      throw new Error('killed');
    };
    await ask(fake, files, { verb: 'register', authId: AUTH_ID });
    expect(saved).toMatchObject({ pendingRegister: { before: [1, 2], node: null } });
    expect((await readState(files.state)).pendingRegister).toEqual({ before: [1, 2], node: null });
    fake.afterStep = () => undefined;
    expect((await ask(fake, files, { verb: 'nodes' })).reply.ok).toBe(true);
    expect(fake.nodes.map((node) => node.name)).toEqual(['hub', 'mac']);
    expect((await readState(files.state)).pendingRegister).toBeNull();
  });

  it('H1: a node that joined with a pre-auth key meanwhile is not taken for the registered one', async () => {
    const { fake, files } = world();
    expect((await ask(fake, files, { verb: 'router-key', site: 'home' })).reply.ok).toBe(true);
    fake.afterStep = (step) => {
      if (step[0] !== 'auth') return;
      fake.join(1, 'router-home');
      throw new Error('killed');
    };
    await ask(fake, files, { verb: 'register', authId: AUTH_ID });
    expect(fake.ran('nodes', 'delete')).toEqual([]);
  });
});

describe('register: an auth ID of a node that was already there (L2)', () => {
  it('L2: a router is refused and left as it was', async () => {
    const { fake, files } = world();
    const router = fake.add('router-home', { tags: ['tag:router', 'tag:site-home'], expiry: null, available: ['10.10.0.0/24'], approved: ['10.10.0.0/24'] });
    fake.waiting.set(AUTH_ID, { name: 'router-home', tags: [], id: router.id });
    const served = await ask(fake, files, { verb: 'register', authId: AUTH_ID });
    expect(served.reply).toMatchObject({ ok: false, refusal: 'not-allowed' });
    expect(fake.node(router.id)).toMatchObject({ tags: ['tag:router', 'tag:site-home'], approved: ['10.10.0.0/24'] });
    expect(fake.ran('nodes', 'delete')).toEqual([]);
    expect(served.log).toBe(`anima-broker verb=register outcome=not-allowed node=${String(router.id)} fixes=0`);
    expect((await readState(files.state)).pendingRegister).toBeNull();
  });

  it('VPN-014: an untagged operator node may still re-register', async () => {
    const { fake, files } = world();
    fake.waiting.set(AUTH_ID, { name: 'mac', tags: [], id: 2 });
    expect((await ask(fake, files, { verb: 'register', authId: AUTH_ID })).reply).toMatchObject({ ok: true, value: { name: 'mac' } });
  });
});
