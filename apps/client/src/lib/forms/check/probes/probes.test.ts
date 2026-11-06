import type { PendingCheckConfigInput } from '$lib/forms/hub-pending';
import type { UpdateCheckInput } from '../model/hub-types';
import { describe, expect, it } from 'vitest';
import { fresh, saved } from '../check.fixture';
import { configChanged, configInput } from '../model/config-input';
import { createCheckInput, updateCheckInput } from '../model/input';
import { validateCheck } from '../model/validate';
import { checkValues } from '../model/values';

/** The update input is typed by the generated schema, which has no SNMP config yet (D-1). */
const sentSnmp = (input: UpdateCheckInput) => (input.config as PendingCheckConfigInput | undefined)?.snmp;
const fields = (errors: readonly { field: string }[]) => errors.map((error) => error.field);

const SNMP_SAVED = saved('SNMP', {
  __typename: 'SnmpConfig',
  port: 1161,
  version: 'V3',
  oid: '1.3.6.1.2.1.1.3.0',
  expected: null,
  username: 'monitor',
  timeoutMs: 4000,
});

describe('CHK-009: defaults and values from a saved check', () => {
  it('a new Docker, systemd, SNMP or Tailscale ping check starts with the K1 defaults', () => {
    const values = checkValues(null, 'SNMP');
    expect(values.docker.timeoutMs).toBe('10000');
    expect(values.systemd.timeoutMs).toBe('10000');
    expect(values.snmp).toMatchObject({ port: '161', version: 'V2C', timeoutMs: '5000', expected: '', storedVersion: null });
    expect(values.tailscalePing).toEqual({ target: '', timeoutMs: '5000' });
  });

  it('a saved check fills its own settings and never a secret', () => {
    const values = checkValues(SNMP_SAVED, 'ICMP');
    expect(values.type).toBe('SNMP');
    expect(values.snmp).toMatchObject({ port: '1161', version: 'V3', username: 'monitor', timeoutMs: '4000', storedVersion: 'V3' });
    expect(values.snmp.community + values.snmp.authPassword + values.snmp.privacyPassword).toBe('');
    const docker = checkValues(saved('DOCKER', { __typename: 'DockerConfig', container: 'web', timeoutMs: 8000 }), 'ICMP');
    expect(docker.docker).toEqual({ container: 'web', timeoutMs: '8000' });
    const unit = checkValues(saved('SYSTEMD', { __typename: 'SystemdConfig', unit: 'nginx.service', timeoutMs: 7000 }), 'ICMP');
    expect(unit.systemd).toEqual({ unit: 'nginx.service', timeoutMs: '7000' });
    const ping = checkValues(saved('TAILSCALE_PING', { __typename: 'TailscalePingConfig', target: null, timeoutMs: 3000 }), 'ICMP');
    expect(ping.tailscalePing).toEqual({ target: '', timeoutMs: '3000' });
  });
});

describe('CHK-009: the config input of each type', () => {
  it('builds the Docker and systemd inputs from trimmed text', () => {
    expect(configInput(fresh('DOCKER', (v) => (v.docker.container = ' web '))).docker).toEqual({ container: 'web', timeoutMs: 10000 });
    expect(configInput(fresh('SYSTEMD', (v) => (v.systemd.unit = 'nginx.service'))).systemd).toEqual({ unit: 'nginx.service', timeoutMs: 10000 });
  });

  it('leaves the Tailscale ping target out when empty', () => {
    expect(configInput(fresh('TAILSCALE_PING')).tailscalePing).toEqual({ timeoutMs: 5000 });
    expect(configInput(fresh('TAILSCALE_PING', (v) => (v.tailscalePing.target = 'nas'))).tailscalePing).toEqual({ target: 'nas', timeoutMs: 5000 });
  });

  it('sends the v2c community only, and the v3 user and passwords only', () => {
    const v2c = fresh('SNMP', (v) => {
      v.snmp.oid = '1.3.6.1.2.1.1.3.0';
      v.snmp.community = 'public';
      v.snmp.username = 'ignored';
    });
    expect(configInput(v2c).snmp).toEqual({ port: 161, version: 'V2C', oid: '1.3.6.1.2.1.1.3.0', community: 'public', timeoutMs: 5000 });
    const v3 = fresh('SNMP', (v) => {
      v.snmp.version = 'V3';
      v.snmp.oid = '1.3.6.1.2.1.1.3.0';
      v.snmp.expected = ' 7 ';
      v.snmp.username = ' monitor ';
      v.snmp.authPassword = 'a';
      v.snmp.privacyPassword = 'p';
      v.snmp.community = 'ignored';
    });
    expect(configInput(v3).snmp).toEqual({
      port: 161,
      version: 'V3',
      oid: '1.3.6.1.2.1.1.3.0',
      expected: '7',
      username: 'monitor',
      authPassword: 'a',
      privacyPassword: 'p',
      timeoutMs: 5000,
    });
  });

  it('creates a check with the config of its type', () => {
    const input = createCheckInput(fresh('DOCKER', (v) => (v.docker.container = 'web')), 'dev-1');
    expect(input.config).toEqual({ docker: { container: 'web', timeoutMs: 10000 } });
  });
});

describe('CHK-012: operator text that looks like shell is refused', () => {
  it.each(['web; rm -rf /', 'web$(id)', 'a b', '-rf', '`id`', 'web\nid', '../x', ''])('Docker container %j', (container) => {
    expect(fields(validateCheck(fresh('DOCKER', (v) => (v.docker.container = container)), null))).toContain('container');
  });

  it.each(['web', 'my_app.1', 'a-b'])('Docker container %j is accepted', (container) => {
    expect(validateCheck(fresh('DOCKER', (v) => (v.docker.container = container)), null)).toEqual([]);
  });

  it.each(['nginx; id', 'nginx.service && id', '$(id)', '-p', 'a b', 'x|y', 'x`id`', ''])('systemd unit %j', (unit) => {
    expect(fields(validateCheck(fresh('SYSTEMD', (v) => (v.systemd.unit = unit)), null))).toContain('unit');
  });

  it.each(['nginx.service', 'nginx', 'getty@tty1.service', 'dev-sda1.device', 'user@1000.service'])('systemd unit %j is accepted', (unit) => {
    expect(validateCheck(fresh('SYSTEMD', (v) => (v.systemd.unit = unit)), null)).toEqual([]);
  });

  it('the Tailscale ping target is a host name or an address', () => {
    expect(fields(validateCheck(fresh('TAILSCALE_PING', (v) => (v.tailscalePing.target = 'nas; id')), null))).toEqual(['target']);
    expect(validateCheck(fresh('TAILSCALE_PING', (v) => (v.tailscalePing.target = '100.64.0.7')), null)).toEqual([]);
    expect(validateCheck(fresh('TAILSCALE_PING'), null)).toEqual([]);
  });

  it('every Phase 2 timeout is bounded', () => {
    expect(fields(validateCheck(fresh('DOCKER', (v) => ((v.docker.container = 'web'), (v.docker.timeoutMs = '5'))), null))).toEqual(['timeoutMs']);
  });
});

describe('SNMP validation', () => {
  const snmp = (change: (values: ReturnType<typeof fresh>['snmp']) => void) => fresh('SNMP', (v) => change(v.snmp));

  it('needs a dotted numeric OID', () => {
    expect(fields(validateCheck(snmp((s) => (s.community = 'public')), null))).toEqual(['oid']);
    for (const oid of ['sysUpTime', '1.3.x', '1', '1.3.6.1;id']) {
      expect(fields(validateCheck(snmp((s) => ((s.oid = oid), (s.community = 'public'))), null))).toEqual(['oid']);
    }
  });

  it('checks the port', () => {
    expect(fields(validateCheck(snmp((s) => ((s.oid = '1.3.6'), (s.community = 'c'), (s.port = '0'))), null))).toEqual(['port']);
  });

  it('a new v2c check needs a community; a new v3 check needs the user and both passwords', () => {
    expect(fields(validateCheck(snmp((s) => (s.oid = '1.3.6')), null))).toEqual(['community']);
    expect(fields(validateCheck(snmp((s) => ((s.oid = '1.3.6'), (s.version = 'V3'))), null))).toEqual(['username', 'authPassword', 'privacyPassword']);
  });
});

describe('SEC-005: SNMP secrets on edit', () => {
  const edit = () => checkValues(SNMP_SAVED, 'ICMP');

  it('empty secrets keep the stored ones, so the edit is valid', () => {
    const values = edit();
    values.snmp.oid = '1.3.6.1.2.1.1.5.0';
    expect(validateCheck(values, edit())).toEqual([]);
  });

  it('an edit that leaves the config alone sends no config', () => {
    const values = edit();
    values.name = 'renamed';
    expect(configChanged(edit(), values)).toBe(false);
    expect(updateCheckInput(edit(), values)).toEqual({ name: 'renamed' });
  });

  it('a changed OID sends the whole config without the empty secrets (keep)', () => {
    const values = edit();
    values.snmp.oid = '1.3.6.1.2.1.1.5.0';
    const sent = sentSnmp(updateCheckInput(edit(), values));
    expect(sent).toMatchObject({ username: 'monitor' });
    expect(sent).not.toHaveProperty('authPassword');
    expect(sent).not.toHaveProperty('privacyPassword');
  });

  it('a port or timeout change on a saved v2c check sends no community key', () => {
    const v2c = () => checkValues(saved('SNMP', { __typename: 'SnmpConfig', port: 161, version: 'V2C', oid: '1.3.6', expected: null, username: null, timeoutMs: 5000 }), 'ICMP');
    const values = v2c();
    values.snmp.port = '1161';
    values.snmp.timeoutMs = '8000';
    const sent = sentSnmp(updateCheckInput(v2c(), values));
    expect(sent).toMatchObject({ port: 1161, timeoutMs: 8000 });
    expect(sent).not.toHaveProperty('community');
  });

  it('switching the version asks for the new version\'s secrets', () => {
    const values = edit();
    values.snmp.version = 'V2C';
    expect(fields(validateCheck(values, edit()))).toEqual(['community']);
  });

  it('typing a new password is sent and changes the config', () => {
    const values = edit();
    values.snmp.authPassword = 'new-secret';
    expect(configChanged(edit(), values)).toBe(true);
    expect(sentSnmp(updateCheckInput(edit(), values))?.authPassword).toBe('new-secret');
  });
});
