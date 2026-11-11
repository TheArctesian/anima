// `harden.sh broker` against the fake VPS (R-055, VP-2, VP-8, D-14).
import { afterEach, describe, expect, setDefaultTimeout, test } from 'bun:test';
import { statSync } from 'node:fs';
import { join } from 'node:path';
import { fakeHost, HARDEN_TIMEOUT_MS, mutatingCalls, removeHosts, type Host } from './vps-host.ts';

setDefaultTimeout(HARDEN_TIMEOUT_MS);
afterEach(removeHosts);

const NEW = 'usr/local/libexec/anima-broker.new';
const BIN = 'usr/local/libexec/anima-broker';
const SOCKET_UNIT = 'etc/systemd/system/anima-broker.socket';
const SERVICE_UNIT = 'etc/systemd/system/anima-broker@.service';

// A stand-in for the compiled broker: `count-nodes` prints 4, `check` answers as the broker does.
function withBinary(host: Host, count = '4'): void {
  host.write(NEW, `#!/bin/sh\ncase "$1" in count-nodes) echo ${count};; check) echo "broker: ok, ${count} nodes";; *) exit 64;; esac\n`, 0o755);
}

describe('harden.sh broker', () => {
  test('VPS-005: a dry run changes nothing', () => {
    const host = fakeHost();
    withBinary(host);
    const before = host.tree();
    const run = host.run(['--dry-run', 'broker']);
    expect(run.code).toBe(0);
    expect(mutatingCalls(run)).toEqual([]);
    expect(host.tree()).toEqual(before);
    expect(run.out).toContain(`[dry-run] write ${join(host.root, 'etc/anima/broker.conf')} (mode 0644)`);
  });

  test('R-055: a missing binary is refused (not in a dry run)', () => {
    const host = fakeHost();
    const run = host.run(['broker']);
    expect(run.code).toBe(2);
    expect(run.out).toContain('anima-broker.new is missing');
    expect(host.exists(SOCKET_UNIT)).toBe(false);
    const dry = host.run(['--dry-run', 'broker']);
    expect(dry.code).toBe(0);
    expect(dry.out).toContain('a real run refuses');
  });

  test('R-055: another architecture is refused', () => {
    const host = fakeHost();
    withBinary(host);
    expect(host.run(['broker'], { FAKE_ARCH: 'armv7l' }).code).toBe(2);
    expect(host.exists(BIN)).toBe(false);
  });

  test('R-055: a binary that cannot count the nodes is refused', () => {
    const host = fakeHost();
    withBinary(host, 'oops');
    const run = host.run(['broker']);
    expect(run.code).toBe(2);
    expect(run.out).toContain('printed no number');
  });

  test('5.1: a new broker.conf is the node count + 5; an existing one is kept', () => {
    const host = fakeHost();
    withBinary(host);
    expect(host.run(['broker']).code).toBe(0);
    expect(host.read('etc/anima/broker.conf')).toBe('max_nodes=9\n');
    withBinary(host);
    host.write('etc/anima/broker.conf', 'max_nodes=30\n');
    const again = host.run(['broker']);
    expect(again.code).toBe(0);
    expect(again.out).toContain('Keeping /etc/anima/broker.conf.');
    expect(host.read('etc/anima/broker.conf')).toBe('max_nodes=30\n');
  });

  test('VP-8: the binary and the units are installed, the socket enabled and checked', () => {
    const host = fakeHost();
    withBinary(host);
    const run = host.run(['broker']);
    expect(run.code).toBe(0);
    expect(run.out).toContain('broker: ok, 4 nodes');
    expect(statSync(join(host.root, BIN)).mode & 0o777).toBe(0o755);
    expect(host.exists(NEW)).toBe(false);
    for (const unit of [SOCKET_UNIT, SERVICE_UNIT]) expect(statSync(join(host.root, unit)).mode & 0o777).toBe(0o644);
    expect(host.read(SOCKET_UNIT)).toContain('MaxConnections=1');
    expect(host.read(SERVICE_UNIT)).toContain('LimitCORE=0');
    expect(run.calls).toEqual(['UNAME -m', 'SYSTEMCTL daemon-reload', 'SYSTEMCTL enable --now anima-broker.socket']);
  });
});
