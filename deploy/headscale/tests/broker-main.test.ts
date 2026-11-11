// One broker request end to end against a fake Headscale (R-055, VP-7, VP-10, LOG-007, BD-9).
import { afterEach, describe, expect, it } from 'bun:test';
import { copyFileSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { serve, type Served } from '../broker/serve.ts';
import { readState, type BrokerFiles } from '../broker/state.ts';
import { FakeHeadscale, KEY } from './broker-fake.ts';

const NOW = new Date('2026-11-10T00:00:00Z');
const AUTH_ID = 'hskey-authreq-Ab3_-Zz9Ab3_-Zz9Ab3_-Zz9';
const folders: string[] = [];
afterEach(() => {
  for (const folder of folders.splice(0)) rmSync(folder, { recursive: true, force: true });
});

function brokerFiles(maxNodes = 10): BrokerFiles {
  const folder = mkdtempSync(join(tmpdir(), 'anima-broker-'));
  folders.push(folder);
  const files = { config: join(folder, 'broker.conf'), policy: join(folder, 'policy.hujson'), state: join(folder, 'state.json') };
  writeFileSync(files.config, `max_nodes=${String(maxNodes)}\n`);
  copyFileSync(new URL('../config/policy.hujson', import.meta.url), files.policy);
  return files;
}

const ask = (fake: FakeHeadscale, files: BrokerFiles, request: object): Promise<Served> => serve(JSON.stringify(request), fake.runner, files, fake.now);

function world(): { fake: FakeHeadscale; files: BrokerFiles } {
  const fake = new FakeHeadscale(NOW);
  fake.add('hub', { tags: ['tag:hub'], expiry: null });
  fake.add('mac');
  return { fake, files: brokerFiles() };
}

describe('serve: register', () => {
  it('VP-7: register then audit undoes a tagged node and replies undone', async () => {
    const { fake, files } = world();
    fake.waiting.set(AUTH_ID, { name: 'sneaky', tags: ['tag:hub'] });
    const served = await ask(fake, files, { verb: 'register', authId: AUTH_ID });
    expect(served.reply).toEqual({ ok: false, refusal: 'undone', detail: 'the new node was deleted: the node had a tag' });
    expect(fake.nodes.map((node) => node.name)).toEqual(['hub', 'mac']);
    expect((await readState(files.state)).pendingRegister).toBeNull();
  });

  it('VP-7: register reads the node id from the reply, not from a list difference', async () => {
    const { fake, files } = world();
    fake.waiting.set(AUTH_ID, { name: 'laptop', tags: [] });
    fake.afterStep = (step) => {
      if (step[0] === 'auth') fake.add('late-router', { tags: ['tag:router'], expiry: null });
    };
    const served = await ask(fake, files, { verb: 'register', authId: AUTH_ID });
    expect(served.reply).toEqual({ ok: true, value: { name: 'laptop', address: '100.64.0.3', keyExpiry: '2027-05-09T00:00:00.000Z' } });
    expect(fake.ran('nodes', 'delete')).toEqual([]);
  });

  it('VPN-014: a renewal by re-login keeps the node id', async () => {
    const { fake, files } = world();
    fake.waiting.set(AUTH_ID, { name: 'mac', tags: [], id: 2 });
    expect((await ask(fake, files, { verb: 'register', authId: AUTH_ID })).reply).toMatchObject({ ok: true, value: { name: 'mac' } });
  });

  it('5.1: a pendingRegister left by a killed request is audited first', async () => {
    const { fake, files } = world();
    const tagged = fake.add('sneaky', { tags: ['tag:hub'], expiry: null });
    writeFileSync(files.state, JSON.stringify({ keys: [], registrations: [NOW.toISOString()], issued: [], pendingRegister: tagged.id }));
    const served = await ask(fake, files, { verb: 'nodes' });
    expect(fake.steps.findIndex((step) => step[1] === 'delete')).toBe(2);
    expect(served.log).toBe('anima-broker verb=nodes outcome=ok node=- fixes=1');
    expect(await readState(files.state)).toMatchObject({ pendingRegister: null, registrations: [NOW.toISOString()] });
  });

  it('5.1: a registration that Headscale refuses is a refusal, and still counted', async () => {
    const { fake, files } = world();
    const served = await ask(fake, files, { verb: 'register', authId: AUTH_ID });
    expect(served.reply).toMatchObject({ ok: false, refusal: 'headscale-failed' });
    expect((await readState(files.state)).registrations).toHaveLength(1);
  });
});

describe('serve: keys and limits', () => {
  it('VPN-001: a router key is returned once, remembered by id, and a wrong router is deleted', async () => {
    const { fake, files } = world();
    const served = await ask(fake, files, { verb: 'router-key', site: 'home' });
    expect(served.reply).toEqual({ ok: true, value: { key: KEY, expiresAt: '2026-11-10T01:00:00.000Z', keyId: 1 } });
    expect((await readState(files.state)).issued).toEqual([{ keyId: 1, verb: 'router-key', site: 'home', at: NOW.toISOString(), passed: false }]);
    fake.join(1, 'router-home', { tags: ['tag:router', 'tag:site-home', 'tag:hub'] });
    await ask(fake, files, { verb: 'nodes' });
    expect(fake.nodes.map((node) => node.name)).toEqual(['hub', 'mac']);
  });

  it('VPN-003: a personal key names the operator by id', async () => {
    const { fake, files } = world();
    expect((await ask(fake, files, { verb: 'personal-key' })).reply).toMatchObject({ ok: true, value: { keyId: 1 } });
    expect(fake.ran('preauthkeys', 'create')).toEqual([['preauthkeys', 'create', '--user=1', '--expiration=1h', '--output=json']]);
  });

  it('5.1: the count is written before the step', async () => {
    const { fake, files } = world();
    fake.afterStep = (step) => {
      if (step[0] === 'preauthkeys') throw new Error('killed');
    };
    const served = await ask(fake, files, { verb: 'personal-key' });
    expect(served.reply).toMatchObject({ ok: false, refusal: 'headscale-failed' });
    expect(fake.keys).toHaveLength(1);
    expect((await readState(files.state)).keys).toEqual([NOW.toISOString()]);
  });

  it('5.1: the state survives between two processes', async () => {
    const { fake, files } = world();
    const script = join(folders[0] ?? '', 'one-request.ts');
    writeFileSync(script, [
      `import { serve } from ${JSON.stringify(new URL('../broker/serve.ts', import.meta.url).pathname)};`,
      `import { FakeHeadscale } from ${JSON.stringify(new URL('./broker-fake.ts', import.meta.url).pathname)};`,
      `const fake = new FakeHeadscale(new Date(${JSON.stringify(NOW.toISOString())}));`,
      `const files = ${JSON.stringify(files)};`,
      'console.log(JSON.stringify((await serve(\'{"verb":"router-key","site":"home"}\', fake.runner, files, fake.now)).reply.ok));',
    ].join('\n'));
    for (let run = 0; run < 3; run += 1) expect(Bun.spawnSync(['bun', script]).stdout.toString().trim()).toBe('true');
    expect((await ask(fake, files, { verb: 'router-key', site: 'home' })).reply).toMatchObject({ ok: false, refusal: 'rate-limited' });
    expect(fake.ran('preauthkeys', 'create')).toEqual([]);
  });

  it('5.1: node-limit refuses before Headscale makes a key', async () => {
    const fake = new FakeHeadscale(NOW);
    fake.add('hub', { tags: ['tag:hub'], expiry: null });
    const files = brokerFiles(1);
    expect((await ask(fake, files, { verb: 'personal-key' })).reply).toMatchObject({ ok: false, refusal: 'node-limit' });
    expect(fake.keys).toEqual([]);
  });

  it('BD-9: an unknown site is refused and not counted', async () => {
    const { fake, files } = world();
    expect((await ask(fake, files, { verb: 'router-key', site: 'mars' })).reply).toMatchObject({ ok: false, refusal: 'unknown-site' });
    expect((await readState(files.state)).keys).toEqual([]);
  });
});

describe('serve: other verbs and the log', () => {
  it('R-055: approve-route approves the waiting route of a router', async () => {
    const { fake, files } = world();
    const router = fake.add('router-home', { tags: ['tag:router', 'tag:site-home'], expiry: null, available: ['10.10.0.0/24'] });
    expect((await ask(fake, files, { verb: 'approve-route', node: router.id, site: 'home' })).reply).toEqual({ ok: true, value: { node: router.id, site: 'home' } });
    expect(router.approved).toEqual(['10.10.0.0/24']);
  });

  it('VP-10: renew sets a staggered expiry and replies the node', async () => {
    const { fake, files } = world();
    const served = await ask(fake, files, { verb: 'renew', node: 2 });
    expect(served.reply).toEqual({ ok: true, value: { name: 'mac', address: '100.64.0.2', keyExpiry: '2027-05-09T00:00:00.000Z' } });
    expect((await ask(fake, files, { verb: 'renew', node: 1 })).reply).toMatchObject({ ok: false, refusal: 'not-allowed' });
  });

  it('VP-7: a bad request never reaches Headscale', async () => {
    const { fake, files } = world();
    const served = await serve('{"verb":"open-device"}', fake.runner, files, NOW);
    expect(served).toEqual({ reply: { ok: false, refusal: 'bad-request', detail: 'the request is not one of the broker verbs' }, log: 'anima-broker verb=- outcome=bad-request node=- fixes=0' });
    expect(fake.steps).toEqual([]);
  });

  it('5.1: an unreadable state is refused, never read as empty', async () => {
    const { fake, files } = world();
    writeFileSync(files.state, '{"keys":[]}');
    expect((await ask(fake, files, { verb: 'personal-key' })).reply).toMatchObject({ ok: false, refusal: 'headscale-failed' });
    expect(fake.keys).toEqual([]);
  });

  it('LOG-007: stderr never holds the key, the auth ID or a node name', async () => {
    const { fake, files } = world();
    fake.waiting.set(AUTH_ID, { name: 'laptop', tags: [] });
    const logs: string[] = [];
    for (const request of [{ verb: 'personal-key' }, { verb: 'router-key', site: 'home' }, { verb: 'register', authId: AUTH_ID }, { verb: 'renew', node: 2 }, { verb: 'nodes' }]) {
      const served = await ask(fake, files, request);
      expect(served.reply.ok).toBe(true);
      logs.push(served.log);
    }
    expect(logs.join('\n')).not.toMatch(new RegExp(`${KEY}|${AUTH_ID}|hskey|laptop|mac|hub`));
    expect(logs[2]).toBe('anima-broker verb=register outcome=ok node=3 fixes=0');
  });

  it('BD-9: nodes replies hold no key fields', async () => {
    const { fake, files } = world();
    fake.add('router-home', { tags: ['tag:router', 'tag:site-home'], expiry: null, preAuthKeyId: 9, available: ['10.10.0.0/24'] });
    const text = JSON.stringify((await ask(fake, files, { verb: 'nodes' })).reply);
    expect(text).toContain('router-home');
    expect(text).not.toMatch(/REDACTED|nodekey|mkey|discokey|pre_auth|preAuthKey|"key"/);
    expect(readFileSync(files.state, 'utf8')).not.toContain(KEY);
  });
});
