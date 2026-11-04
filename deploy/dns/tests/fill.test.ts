import { afterEach, describe, expect, test } from 'bun:test';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { isPublicAddress, isVpnAddress } from '../address.ts';
import { type Env, FillError, fillRecords } from '../fill.ts';

const dnsDir = join(import.meta.dir, '..');
const template = readFileSync(join(dnsDir, 'anima.ing.records.json'), 'utf8');
const good: Env = { VPS_PUBLIC_IPV4: '45.10.20.30', HUB_VPN_IPV4: '100.64.0.7' };

interface Filled {
  records: { name: string; content?: string }[];
}

function fillError(names: readonly string[], env: Env): FillError {
  try {
    fillRecords(template, names, env);
  } catch (error) {
    if (error instanceof FillError) return error;
    throw error;
  }
  throw new Error('fillRecords accepted the input');
}

const dirs: string[] = [];
afterEach(() => {
  for (const dir of dirs.splice(0)) rmSync(dir, { recursive: true, force: true });
});

describe('fill checks', () => {
  test('DNS-005 a hub address outside 100.64.0.0/10 or not a strict dotted quad is rejected', () => {
    for (const address of ['10.10.0.5', '100.63.255.255', '100.128.0.1', '192.168.1.10', '8.8.8.8', '100.64.0', '100.64.0.1.2', '100.064.0.1', '100.64.0.256', ' 100.64.0.1', '']) {
      expect(isVpnAddress(address)).toBe(false);
    }
  });

  test('DNS-005 a hub address inside 100.64.0.0/10 is accepted', () => {
    for (const address of ['100.64.0.0', '100.64.0.1', '100.100.7.3', '100.127.255.255']) {
      expect(isVpnAddress(address)).toBe(true);
    }
  });

  test('DNS-004 a VPS address that is private, CGNAT, loopback, link-local, documentation or reserved is rejected', () => {
    for (const address of ['10.0.0.1', '172.16.0.1', '172.31.255.255', '192.168.0.1', '100.64.0.1', '100.127.0.1', '127.0.0.1', '169.254.1.1', '192.0.2.10', '198.51.100.10', '203.0.113.10', '0.0.0.0', '224.0.0.1', '255.255.255.255', '198.18.0.1', '198.19.255.255', '192.0.0.8', '45.10.20', '1.2.3.4.5', '045.10.20.30']) {
      expect(isPublicAddress(address)).toBe(false);
    }
  });

  test('DNS-004 a public VPS address is accepted', () => {
    for (const address of ['45.10.20.30', '8.8.8.8', '172.15.255.255', '172.32.0.1', '100.63.255.255', '100.128.0.1']) {
      expect(isPublicAddress(address)).toBe(true);
    }
  });
});

describe('fillRecords', () => {
  test('DNS-004 DNS-005 valid values fill hs and hub and leave no placeholder', () => {
    const filled = fillRecords(template, [], good);
    expect(filled).not.toContain('${');
    const records = (JSON.parse(filled) as Filled).records;
    expect(records.find((r) => r.name === 'hs.anima.ing')?.content).toBe('45.10.20.30');
    expect(records.find((r) => r.name === 'hub.anima.ing')?.content).toBe('100.64.0.7');
  });

  test('DNS-005 HUB_VPN_IPV4 10.10.0.5 is refused', () => {
    expect(fillError([], { ...good, HUB_VPN_IPV4: '10.10.0.5' }).message).toContain('HUB_VPN_IPV4');
  });

  test('DNS-004 an empty or unset VPS_PUBLIC_IPV4 is refused', () => {
    expect(fillError(['hs.anima.ing'], { ...good, VPS_PUBLIC_IPV4: '' }).message).toContain('unset or empty');
    expect(fillError(['hs.anima.ing'], { HUB_VPN_IPV4: good.HUB_VPN_IPV4 }).message).toContain('VPS_PUBLIC_IPV4');
  });

  test('DNS-004 a private VPS_PUBLIC_IPV4 is refused', () => {
    expect(fillError(['hs.anima.ing'], { ...good, VPS_PUBLIC_IPV4: '192.168.1.2' }).message).toContain('VPS_PUBLIC_IPV4');
  });

  test('DNS-004 selecting hs needs only VPS_PUBLIC_IPV4', () => {
    const records = (JSON.parse(fillRecords(template, ['hs.anima.ing'], { VPS_PUBLIC_IPV4: '45.10.20.30' })) as Filled).records;
    expect(records.map((r) => r.name)).toEqual(['hs.anima.ing']);
  });

  test('DNS-005 an unknown record name is refused', () => {
    expect(fillError(['pihole.net.anima.ing'], good)).toBeInstanceOf(FillError);
  });

  test('LOG-007 a refusal message never repeats the rejected value', () => {
    const pasted = 'not-an-address-maybe-a-pasted-secret';
    expect(fillError([], { ...good, HUB_VPN_IPV4: pasted }).message).not.toContain(pasted);
  });
});

describe('fill.ts command', () => {
  function run(args: readonly string[], env: Env): { code: number; stderr: string } {
    const result = Bun.spawnSync([process.execPath, join(dnsDir, 'fill.ts'), ...args], {
      env: { PATH: process.env.PATH, ...env },
      stdout: 'pipe',
      stderr: 'pipe',
    });
    return { code: result.exitCode, stderr: result.stderr.toString() };
  }

  test('DNS-005 refuses with a non-zero exit and writes nothing when HUB_VPN_IPV4 is outside the VPN range', () => {
    const dir = mkdtempSync(join(tmpdir(), 'dns-fill-'));
    dirs.push(dir);
    const out = join(dir, 'filled.json');
    const result = run(['--out', out], { ...good, HUB_VPN_IPV4: '10.10.0.5' });
    expect(result.code).not.toBe(0);
    expect(result.stderr).toContain('HUB_VPN_IPV4 must be');
    expect(existsSync(out)).toBe(false);
  });

  test('DNS-004 DNS-005 writes the filled records to the named file when every check passes', () => {
    const dir = mkdtempSync(join(tmpdir(), 'dns-fill-'));
    dirs.push(dir);
    const out = join(dir, 'filled.json');
    expect(run(['--out', out], good).code).toBe(0);
    expect(readFileSync(out, 'utf8')).toBe(fillRecords(template, [], good));
  });
});
