// `harden.sh broker` against the fake VPS (R-055, VP-2, VP-8, D-14, H2, L5, M3).
import { afterEach, describe, expect, setDefaultTimeout, test } from 'bun:test';
import { readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { fakeHost, HARDEN_TIMEOUT_MS, mutatingCalls, removeHosts, type Host } from './vps-host.ts';

setDefaultTimeout(HARDEN_TIMEOUT_MS);
afterEach(removeHosts);

const NEW = 'usr/local/libexec/anima-broker.new';
const BIN = 'usr/local/libexec/anima-broker';
const UNIT_DIR = 'etc/systemd/system';
const SOCKET_UNIT = `${UNIT_DIR}/anima-broker.socket`;
const SERVICE_UNIT = `${UNIT_DIR}/anima-broker@.service`;
const AUDIT_UNIT = `${UNIT_DIR}/anima-broker-audit.service`;
const TIMER_UNIT = `${UNIT_DIR}/anima-broker-audit.timer`;
const LOCK = '/usr/bin/flock -w';
const LOCK_FILE = '/var/lib/anima-broker/lock';

// The read-only user and group lookups of the hub-user step.
const notLookup = (call: string): boolean => !call.startsWith('GETENT ');

const sha256 = (text: string): string => new Bun.CryptoHasher('sha256').update(text).digest('hex');

// A stand-in for the compiled broker: `count-nodes` prints 4, `check` answers as the broker does.
function withBinary(host: Host, count = '4'): string {
  const text = `#!/bin/sh\ncase "$1" in count-nodes) echo ${count};; check) echo "broker: ok, ${count} nodes";; *) exit 64;; esac\n`;
  host.write(NEW, text, 0o755);
  return `--sha256=${sha256(text)}`;
}

describe('harden.sh broker: refusals', () => {
  test('VPS-005: a dry run changes nothing and runs nothing from the new binary', () => {
    const host = fakeHost();
    const sum = withBinary(host);
    const before = host.tree();
    const run = host.run(['--dry-run', sum, 'broker']);
    expect(run.code).toBe(0);
    expect(mutatingCalls(run)).toEqual([]);
    expect(host.tree()).toEqual(before);
    expect(run.out).toContain('anima-broker.new matches --sha256.');
    expect(run.out).toContain('[dry-run] /usr/local/libexec/anima-broker.new count-nodes');
    expect(run.out).toContain(`[dry-run] write ${join(host.root, 'etc/anima/broker.conf')} with max_nodes = the node count + 5`);
  });

  test('R-055: a missing binary is refused (not in a dry run)', () => {
    const host = fakeHost();
    const run = host.run(['--sha256=' + 'a'.repeat(64), 'broker']);
    expect(run.code).toBe(2);
    expect(run.out).toContain('anima-broker.new is missing');
    expect(host.exists(SOCKET_UNIT)).toBe(false);
    const dry = host.run(['--dry-run', 'broker']);
    expect(dry.code).toBe(0);
    expect(dry.out).toContain('a real run refuses');
  });

  test('L5: a binary without --sha256, or with another SHA-256, is refused before it runs', () => {
    const host = fakeHost();
    withBinary(host);
    for (const args of [['broker'], [`--sha256=${'0'.repeat(64)}`, 'broker'], ['--sha256=ABC', 'broker']]) {
      const run = host.run(args);
      expect([args, run.code]).toEqual([args, 2]);
      expect(host.exists(BIN)).toBe(false);
    }
    expect(host.run([`--sha256=${'0'.repeat(64)}`, 'broker']).out).toContain('does not match --sha256');
  });

  test('R-055: another architecture is refused', () => {
    const host = fakeHost();
    const sum = withBinary(host);
    expect(host.run([sum, 'broker'], { FAKE_ARCH: 'armv7l' }).code).toBe(2);
    expect(host.exists(BIN)).toBe(false);
  });

  test('R-055: a binary that cannot count the nodes is refused', () => {
    const host = fakeHost();
    const sum = withBinary(host, 'oops');
    const run = host.run([sum, 'broker']);
    expect(run.code).toBe(2);
    expect(run.out).toContain('printed no number');
  });

});

describe('harden.sh broker: install', () => {
  test('5.1: a new broker.conf is the node count + 5; an existing one is kept', () => {
    const host = fakeHost();
    expect(host.run([withBinary(host), 'broker']).code).toBe(0);
    expect(host.read('etc/anima/broker.conf')).toBe('max_nodes=9\n');
    const sum = withBinary(host);
    host.write('etc/anima/broker.conf', 'max_nodes=30\n');
    const again = host.run([sum, 'broker']);
    expect(again.code).toBe(0);
    expect(again.out).toContain('Keeping /etc/anima/broker.conf.');
    expect(host.read('etc/anima/broker.conf')).toBe('max_nodes=30\n');
  });

  test('VP-8, H2: the binary and the four units are installed, the socket and the audit timer restarted and checked', () => {
    const host = fakeHost();
    const run = host.run([withBinary(host), 'broker']);
    expect(run.code).toBe(0);
    expect(run.out).toContain('broker: ok, 4 nodes');
    expect(statSync(join(host.root, BIN)).mode & 0o777).toBe(0o755);
    expect(host.exists(NEW)).toBe(false);
    expect(readdirSync(join(host.root, 'usr/local/libexec'))).toEqual(['anima-broker']);
    for (const unit of [SOCKET_UNIT, SERVICE_UNIT, AUDIT_UNIT, TIMER_UNIT]) expect(statSync(join(host.root, unit)).mode & 0o777).toBe(0o644);
    expect(host.read(SOCKET_UNIT)).toContain('MaxConnections=1');
    expect(host.read(SERVICE_UNIT)).toContain('LimitCORE=0');
    expect(host.read(TIMER_UNIT)).toContain('OnUnitActiveSec=5min');
    expect(host.read(TIMER_UNIT)).toContain('OnBootSec=');
    expect(run.calls.filter(notLookup)).toEqual([
      'UNAME -m',
      'GROUPADD --system --gid 10001 anima-hub',
      'USERADD --system --uid 10001 --gid 10001 --no-create-home --home-dir /nonexistent --shell /sbin/nologin anima-hub',
      'SYSTEMCTL daemon-reload',
      'SYSTEMCTL enable anima-broker.socket anima-broker-audit.timer',
      'SYSTEMCTL restart anima-broker.socket anima-broker-audit.timer',
    ]);
  });

  test('R-055: the socket belongs to anima-hub, the hub container\'s 10001; an existing one is kept, a clash refused', () => {
    const hub = 'anima-hub:x:10001:10001::/nonexistent:/sbin/nologin';
    const host = fakeHost();
    const kept = host.run([withBinary(host), 'broker'], { FAKE_HUB: hub });
    expect(kept.code).toBe(0);
    expect(kept.calls.filter((call) => /^(GROUPADD|USERADD) /.test(call))).toEqual([]);
    expect(host.read(SOCKET_UNIT)).toContain('SocketUser=anima-hub\nSocketGroup=anima-hub\n');
    const other = fakeHost();
    const clash = other.run([withBinary(other), 'broker'], { FAKE_HUB: 'builder:x:10001:10001::/home/builder:/bin/sh' });
    expect(clash.code).toBe(2);
    expect(clash.out).toContain('the broker socket needs anima-hub:10001 for the hub');
    expect(mutatingCalls(clash)).toEqual([]);
    expect(other.exists(BIN)).toBe(false);
    expect(other.exists('etc/anima/broker.conf')).toBe(false);
  });

  test('L5: an upgrade stops the socket and the timer before it replaces the binary', () => {
    const host = fakeHost();
    expect(host.run([withBinary(host), 'broker']).code).toBe(0);
    const run = host.run([withBinary(host), 'broker'], { FAKE_HUB: 'anima-hub:x:10001:10001::/nonexistent:/sbin/nologin' });
    expect(run.code).toBe(0);
    expect(run.calls.filter(notLookup).slice(0, 3)).toEqual(['UNAME -m', 'SYSTEMCTL stop anima-broker.socket', 'SYSTEMCTL stop anima-broker-audit.timer']);
  });

});

describe('harden.sh broker: units', () => {
  test('H2: the request and the audit take the same lock, and the audit runs the audit entry point', () => {
    const host = fakeHost();
    expect(host.run([withBinary(host), 'broker']).code).toBe(0);
    const exec = (unit: string): string => host.read(unit).split('\n').find((line) => line.startsWith('ExecStart=')) ?? '';
    expect(exec(SERVICE_UNIT)).toMatch(new RegExp(`^ExecStart=${LOCK} \\d+ ${LOCK_FILE} /usr/local/libexec/anima-broker$`));
    expect(exec(AUDIT_UNIT)).toMatch(new RegExp(`^ExecStart=${LOCK} \\d+ ${LOCK_FILE} /usr/local/libexec/anima-broker audit$`));
  });

  test('H2, M3: the audit unit has the request unit\'s user and sandbox, and both have the M3 lines', () => {
    const host = fakeHost();
    expect(host.run([withBinary(host), 'broker']).code).toBe(0);
    const sandbox = (unit: string): string[] => {
      const lines = host.read(unit).split('\n');
      return lines.slice(lines.indexOf('NoNewPrivileges=yes')).filter((line) => line !== '' && !line.startsWith('#'));
    };
    expect(sandbox(AUDIT_UNIT)).toEqual(sandbox(SERVICE_UNIT));
    for (const line of ['ProtectProc=invisible', 'ProcSubset=pid', 'SystemCallFilter=@system-service', 'SystemCallFilter=~@privileged', 'CapabilityBoundingSet=', 'PrivateNetwork=yes']) {
      expect(sandbox(SERVICE_UNIT)).toContain(line);
    }
    for (const line of ['User=root', 'StateDirectory=anima-broker', 'UMask=0077', 'LimitCORE=0']) expect(host.read(AUDIT_UNIT)).toContain(`${line}\n`);
  });
});
