import { describe, expect, it } from 'vitest';
import { check, device } from '../page.fixture.svelte';
import { actionWords, connectActions, headerLine, protocolButton, statusReason } from './header';

describe('headerLine (DEV-020, D-8)', () => {
  it('DEV-020: the IP after the host and the OS logo as the mark', () => {
    const line = headerLine(device());
    expect(line.ip).toBe('192.168.1.20');
    expect(line.osIcon).toBe('simple-icons:linux');
  });

  it.each([null, '', '192.168.1.20'])('D-8: the IP is left out when it is %j and the host is 192.168.1.20', (ip) => {
    expect(headerLine(device({ host: '192.168.1.20', ip })).ip).toBeNull();
  });

  it.each([
    [{ enabled: true, hostKey: device().ssh.hostKey }, 'connect'],
    [{ enabled: true, hostKey: null }, 'test'],
    [{ enabled: false, hostKey: null }, 'set-up'],
  ] as const)('D-8: SSH %j does %s', (ssh, action) => {
    expect(headerLine(device({}, ssh)).ssh).toBe(action);
  });
});

describe('protocolButton (DEV-022)', () => {
  it.each([
    ['SSH', 'connect', 'SSH to nas', null],
    ['RDP', 'connect', 'RDP to nas', null],
    ['SSH', 'test', 'Test the SSH connection', 'test'],
    ['SSH', 'set-up', 'Set up SSH', 'ssh'],
    ['RDP', 'set-up', 'Set up RDP', 'rdp'],
  ] as const)('DEV-022: %s in mode %s is named %s and opens %s', (protocol, mode, label, section) => {
    expect(protocolButton(protocol, mode, 'nas')).toEqual({ label, section });
  });

  it('D-4: the line gives RDP connect when on, set-up when off, and VNC only when on', () => {
    expect(headerLine(device()).rdp).toBe('set-up');
    expect(headerLine(device()).vnc).toBe(false);
    const on = headerLine(device({ rdp: { ...device().rdp, enabled: true }, vnc: { ...device().vnc, enabled: true } }));
    expect([on.rdp, on.vnc]).toEqual(['connect', true]);
  });
});

describe('connectActions (D16, DEV-022)', () => {
  const plan = (shown = device()) => {
    const { main, others } = connectActions(headerLine(shown), shown.name);
    return [actionWords(main), ...others.map(actionWords)];
  };

  it('D16: the first protocol that connects leads, the others that connect come next, then the set-ups', () => {
    expect(plan()).toEqual(['SSH', 'Set up RDP']);
    expect(plan(device({ rdp: { ...device().rdp, enabled: true }, vnc: { ...device().vnc, enabled: true } }))).toEqual(['SSH', 'RDP', 'VNC']);
    expect(plan(device({ rdp: { ...device().rdp, enabled: true } }, { enabled: false, hostKey: null }))).toEqual(['RDP', 'Set up SSH']);
  });

  it('D16: with nothing that connects, SSH\'s test or set-up leads', () => {
    expect(plan(device({}, { hostKey: null }))).toEqual(['Test SSH', 'Set up RDP']);
    expect(plan(device({}, { enabled: false, hostKey: null }))).toEqual(['Set up SSH', 'Set up RDP']);
  });
});

describe('statusReason (D16, D18)', () => {
  it('D16: an up device has no reason', () => {
    expect(statusReason(device(), [check()])).toBeNull();
  });

  it('D16: a down device reads its worst check\'s last message as a sentence, once its checks have arrived', () => {
    const down = device({ status: 'DOWN' });
    expect(statusReason(down, null)).toBeNull();
    const failing = check({ status: 'DOWN', lastHeartbeat: { at: '2026-11-04T10:00:00Z', status: 'DOWN', responseTimeMs: null, message: 'connection refused' } });
    expect(statusReason(down, [failing])).toBe('Connection refused.');
    expect(statusReason(down, [check({ status: 'DOWN', lastHeartbeat: null })])).toBe('Ping is down.');
  });

  it('D16: maintenance needs no checks to say so', () => {
    expect(statusReason(device({ status: 'MAINTENANCE' }), null)).toBe('In maintenance.');
  });
});
