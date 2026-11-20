import { expect, it } from 'vitest';
import { aboutFacts, daemonText, waysIn, webDetail, type SideDevice } from './side';

const DEVICE: SideDevice = {
  id: 'd1',
  name: 'pop-os',
  status: 'UP',
  host: 'pop-os.lan',
  ip: null,
  os: 'LINUX',
  distribution: 'pop',
  accessWarnings: ['UNENCRYPTED_VNC'],
  webInterfaces: [{ id: 'w1', deviceId: 'd1', label: 'Grafana', url: 'https://pop-os.lan:3000/', openIn: 'ANIMA_TAB', pinnedFingerprintSha256: null, pinnedAt: null }],
  kind: 'Workstation',
  tags: ['lab'],
  metricsSource: 'DAEMON',
  metricsStale: false,
  daemonVersion: '0.8.14',
  sessionLimit: 4,
  ssh: { enabled: true, hostKey: { algorithm: 'ssh-ed25519', fingerprintSha256: 'abc', firstSeenAt: '2026-11-01T00:00:00Z' }, port: 22, username: 'okita' },
  rdp: { enabled: false, port: 3389 },
  vnc: { enabled: true, username: null, port: 5900 },
};

it('R-224: each way in is a row in the order SSH, RDP, VNC, then each web interface, with what its button does', () => {
  expect(waysIn(DEVICE, true).map((way) => [way.name, way.detail, way.button, way.main])).toEqual([
    ['SSH', 'okita · port 22', 'Open', true],
    ['RDP', 'Not set up', 'Set up', false],
    ['VNC', 'port 5900 · not encrypted', 'Open', false],
    ['Grafana', 'https, port 3000', 'Open', false],
  ]);
});

it('R-224, DEV-022: SSH with no accepted host key offers the test, and no row is the main one', () => {
  const ways = waysIn({ ...DEVICE, ssh: { ...DEVICE.ssh, hostKey: null }, vnc: { ...DEVICE.vnc, enabled: false } }, true);
  expect(ways.map((way) => [way.name, way.detail, way.button, way.main])).toEqual([
    ['SSH', 'okita · port 22 · host key not accepted yet', 'Test', false],
    ['RDP', 'Not set up', 'Set up', false],
    ['Grafana', 'https, port 3000', 'Open', false],
  ]);
});

it('R-224: a web interface has no button where the page cannot open one', () => {
  expect(waysIn(DEVICE, false).at(-1)?.button).toBeNull();
});

it('R-224: the detail of a web interface is its scheme and its port, with the usual port when the address names none', () => {
  expect(webDetail('http://nas.lan/')).toBe('http, port 80');
  expect(webDetail('https://nas.lan/admin')).toBe('https, port 443');
  expect(webDetail('not an address')).toBe('not an address');
});

it('R-224, MET-018: the daemon line says the version and whether it sends, or what reads the metrics in its place', () => {
  expect(daemonText(DEVICE)).toBe('0.8.14, sending metrics');
  expect(daemonText({ ...DEVICE, metricsStale: true })).toBe('0.8.14, not sending now');
  expect(daemonText({ ...DEVICE, daemonVersion: null })).toBe('Version not known, sending metrics');
  expect(daemonText({ ...DEVICE, metricsSource: 'SSH' })).toBe('Not installed. Metrics are read over SSH');
  expect(daemonText({ ...DEVICE, metricsSource: 'NONE' })).toBe('Not installed');
});

it('R-224, RA-036: the card About gives the system, the kind, the daemon and the session limit', () => {
  expect(aboutFacts(DEVICE).map((fact) => fact.text)).toEqual(['Pop!_OS', 'Workstation', '0.8.14, sending metrics', 'At most 4 at once']);
  expect(aboutFacts({ ...DEVICE, sessionLimit: null }).at(-1)?.text).toBe('No limit');
});
