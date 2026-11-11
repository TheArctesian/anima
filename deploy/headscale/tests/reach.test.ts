// The reachability table and its VPN-004 properties (VPN-012).
import { describe, expect, it } from 'bun:test';
import {
  agentOverreachRows,
  crossSiteRows,
  deviceOverreachRows,
  lanToClientRows,
  parties,
  probedPorts,
  reachMarkdown,
  reachTable,
} from '../src/reach.ts';
import { nodes, policy, raw, withGrants } from './policy-fixture.ts';

const table = reachTable(policy, nodes);

describe('reachTable', () => {
  it('VPN-012: no router reaches another site', () => {
    expect(table.some((row) => row.from.kind === 'router' && row.to.kind === 'lan')).toBe(true);
    expect(crossSiteRows(table)).toEqual([]);
  });

  it('VPN-012: no LAN reaches a client', () => {
    expect(table.some((row) => row.from.kind === 'lan' && row.to.kind === 'client')).toBe(true);
    expect(lanToClientRows(table)).toEqual([]);
  });

  it('VPN-012: an agent reaches only the hub on tcp 443', () => {
    expect(agentOverreachRows(table)).toEqual([]);
    expect(table.some((row) => row.from.kind === 'agent' && row.allowed)).toBe(true);
  });

  it('VPN-012: a device /32 is reached only from the hub on tcp', () => {
    expect(deviceOverreachRows(table)).toEqual([]);
    expect(table.some((row) => row.to.kind === 'device' && row.allowed)).toBe(true);
  });

  it('VPN-012: a client reaches the hub', () => {
    const row = table.find((entry) => entry.from.kind === 'client' && entry.to.kind === 'hub' && entry.proto === 'tcp' && entry.port === 443);
    expect(row?.allowed).toBe(true);
  });

  it('VPN-012: a widened grant fails the table', () => {
    const widened = withGrants([...(raw.grants as unknown[]), { src: ['tag:site-home'], dst: ['site-office2'], ip: ['tcp:22'] }]);
    expect(crossSiteRows(reachTable(widened, nodes))).not.toEqual([]);
  });
});

describe('table inputs', () => {
  it('VPN-004: probed ports hold each rule edge and its neighbours, sorted and inside 1-65535', () => {
    const ports = probedPorts(policy);
    expect(ports).toEqual([...ports].sort((a, b) => a - b));
    expect(ports).toEqual(expect.arrayContaining([1, 21, 22, 23, 442, 443, 444, 5899, 5900, 5901, 65_535]));
    expect(ports.every((port) => port >= 1 && port <= 65_535)).toBe(true);
  });

  it('VPN-004: a device node is a device and not a client; each site has one LAN party', () => {
    const everyone = parties(policy, nodes);
    expect(everyone).toContainEqual({ kind: 'device', node: 'mac' });
    expect(everyone).not.toContainEqual({ kind: 'client', node: 'mac' });
    expect(everyone).toContainEqual({ kind: 'router', node: 'router-home', site: 'home' });
    expect(everyone.filter((party) => party.kind === 'lan').map((party) => party.address)).toEqual(['10.10.0.5', '10.20.0.5', '10.30.0.5']);
  });

  it('VPN-012: the Markdown lists allowed pairs only, sorted', () => {
    const text = reachMarkdown(table);
    expect(text).toContain('| client laptop | hub hub | tcp | 22, 443 |');
    const rows = text.trim().split('\n').slice(2);
    expect(rows).toEqual([...rows].sort());
  });
});
