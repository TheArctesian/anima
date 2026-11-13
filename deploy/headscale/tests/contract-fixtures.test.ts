// The fixtures that test/contract.sh captured from Headscale 0.29.7 and Tailscale: the parsers
// and the hub's reading rules must accept the real shapes (VPN-009, VPN-013, VPN-014).
import { describe, expect, it } from 'bun:test';
import { parseNodes, parseRoutes } from '../src/inventory.ts';
import { PREAUTH_PLACEHOLDER } from '../src/redact.ts';

const deployFixture = (name: string): Promise<unknown> => Bun.file(new URL(`fixtures/${name}`, import.meta.url)).json();
const hubFixture = (name: string): Promise<unknown> => Bun.file(new URL(`../../../crates/hub/src/fleet/vpn/tests/fixtures/${name}`, import.meta.url)).json();

interface Peer {
  readonly HostName: string;
  readonly Active: boolean;
  readonly CurAddr: string;
}
const routerIn = (status: unknown): Peer | undefined => Object.values((status as { Peer: Record<string, Peer> }).Peer).find((peer) => peer.HostName === 'router-home');

describe('captured fixtures', () => {
  it('VPN-013: the real node list parses, without any key', async () => {
    const raw = await deployFixture('nodes.json');
    const nodes = parseNodes(raw);
    expect(nodes.map((node) => node.name)).toEqual(['hub', 'router-home', 'mac', 'stranger', 'odd', 'mac2']);
    expect(nodes.find((node) => node.name === 'hub')).toMatchObject({ user: 'tagged-devices', tags: ['tag:hub'], expiry: null, preAuthKeyId: 1 });
    expect(nodes.find((node) => node.name === 'mac')?.expiry).toBeInstanceOf(Date);
    expect(JSON.stringify(raw)).toContain(PREAUTH_PLACEHOLDER);
    expect(JSON.stringify(nodes)).not.toMatch(/REDACTED|nodekey:|mkey:|discokey:/);
  });

  it('VPN-013: the real route list parses', async () => {
    expect(parseRoutes(await deployFixture('routes.json'))).toEqual([{ id: 2, approved: ['10.10.0.0/24'], available: ['10.10.0.0/24'] }]);
  });

  it('VPN-014: a registered node (auth register) has an expiry and no pre-auth key', async () => {
    for (const name of ['register.json', 'register-again.json']) {
      const reply = (await deployFixture(name)) as { id: number; name: string; expiry: { seconds: number }; pre_auth_key?: unknown };
      expect(reply).toMatchObject({ id: 6, name: 'mac2' });
      expect(reply.expiry.seconds).toBeGreaterThan(0);
      expect(reply.pre_auth_key).toBeUndefined();
    }
  });

  it('VPN-014: preauthkeys create returns id, key and expiration', async () => {
    expect(await deployFixture('preauthkey.json')).toMatchObject({ id: 1, key: PREAUTH_PLACEHOLDER, acl_tags: ['tag:hub'] });
  });

  it('VPN-009: a router peer is Active with a CurAddr after a ping, and idle again after 60 s (BD-13)', async () => {
    expect(routerIn(await hubFixture('captured-status-idle.json'))).toMatchObject({ Active: false, CurAddr: '' });
    const active = routerIn(await hubFixture('captured-status-active.json'));
    expect(active?.Active).toBe(true);
    expect(active?.CurAddr).not.toBe('');
    expect(routerIn(await hubFixture('captured-status-after-60s.json'))).toMatchObject({ Active: false, CurAddr: '' });
  });
});
