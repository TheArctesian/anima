// The broker's reading of Headscale 0.29.7 replies (BD-9), against the contract harness captures.
import { describe, expect, it } from 'bun:test';
import { newKeyOf, nodeCountOf, nodeIdsOf, nodesReply, operatorIdOf, registered, registeredIdOf, tailnetOf } from '../broker/nodes.ts';
import { InventoryError } from '../src/inventory.ts';
import { parsePrefix, prefixText } from '../src/policy.ts';
import { view } from './broker-fixture.ts';

const fixture = (name: string): Promise<unknown> => Bun.file(new URL(`fixtures/${name}`, import.meta.url)).json();

describe('broker replies', () => {
  it('BD-9: the nodes reply holds no key field and merges the routes', async () => {
    const tailnet = tailnetOf(await fixture('nodes.json'), await fixture('routes.json'));
    const reply = nodesReply(tailnet, view);
    expect(JSON.stringify(reply)).not.toMatch(/REDACTED|nodekey|mkey|discokey|pre_auth|preAuthKey|"key"/);
    expect(reply.nodes.find((node) => node.name === 'router-home')).toMatchObject({ id: 2, tags: ['tag:router', 'tag:site-home'], approved: ['10.10.0.0/24'], available: ['10.10.0.0/24'], expiry: null });
    expect(reply.nodes.find((node) => node.name === 'mac')?.expiry).toBe('2027-05-10T06:26:48.000Z');
    expect(reply.sites).toContainEqual({ name: 'home', subnet: '10.10.0.0/24' });
  });

  it('VPN-014: Headscale prints null for an empty list', () => {
    expect(tailnetOf(null, [])).toEqual({ nodes: [], routes: [], unreadable: [] });
    expect(operatorIdOf(null)).toBeNull();
  });

  it('L3: a node the broker cannot read is set aside and counted; the others are read', async () => {
    const nodes = (await fixture('nodes.json')) as Record<string, unknown>[];
    const odd = [...nodes, { ...nodes[0], id: 40, expiry: { seconds: -62_135_596_800 } }, { id: 'x' }];
    const tailnet = tailnetOf(odd, []);
    expect(tailnet.nodes).toHaveLength(nodes.length);
    expect(tailnet.unreadable).toEqual([40, null]);
    expect(nodeCountOf(tailnet)).toBe(nodes.length + 2);
    expect(nodeIdsOf(tailnet)).toContain(40);
    expect(() => tailnetOf({ nodes: [] }, [])).toThrow(InventoryError);
  });

  it('VPN-014: the operator user id comes from users list', () => {
    expect(operatorIdOf([{ id: 2, name: 'intruder' }, { id: 1, name: 'operator', created_at: { seconds: 1 } }])).toBe(1);
    expect(operatorIdOf([{ id: 2, name: 'intruder' }])).toBeNull();
    expect(() => operatorIdOf({ id: 1 })).toThrow(InventoryError);
  });

  it('VPN-014: a new key reply gives the id, the key and the expiry (preauthkey.json)', async () => {
    const key = newKeyOf(await fixture('preauthkey.json'));
    expect(key).toEqual({ id: 1, key: 'REDACTED-PREAUTH-KEY', expiresAt: new Date(1_794_382_003_000) });
    expect(() => newKeyOf({ id: 1, expiration: { seconds: 1 } })).toThrow(InventoryError);
    expect(() => newKeyOf({ error: 'creating key' })).toThrow(InventoryError);
  });

  it('VPN-014: auth register returns the node; a renewal by re-login has the same id', async () => {
    expect(registeredIdOf(await fixture('register.json'))).toBe(6);
    expect(registeredIdOf(await fixture('register-again.json'))).toBe(6);
    expect(() => registeredIdOf({ error: 'node not found in registration cache' })).toThrow(InventoryError);
  });

  it('VP-10: the reply for a node is its name, IPv4 address and expiry', async () => {
    const mac = tailnetOf(await fixture('nodes.json'), []).nodes.find((node) => node.id === 3);
    if (mac === undefined) throw new Error('fixture lacks node 3');
    expect(registered(mac)).toEqual({ name: 'mac', address: '100.64.0.3', keyExpiry: '2027-05-10T06:26:48.000Z' });
  });

  it('a prefix prints back as it parses', () => {
    for (const text of ['10.10.0.0/24', '100.64.0.2/32', '255.255.255.254/31', '128.0.0.0/1']) expect(prefixText(parsePrefix(text))).toBe(text);
  });
});
