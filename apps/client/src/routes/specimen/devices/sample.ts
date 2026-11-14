/**
 * The mockup's twelve devices (UR-7, D18), synthetic: name, host, OS, distribution, status, 24-hour uptime, readings
 * (CPU, memory, disk, VRAM), the ways to connect, and the last heartbeat's message.
 */
export const SAMPLE: readonly (readonly [
  string,
  string,
  string,
  string | null,
  string,
  number | null,
  readonly number[] | null,
  readonly string[],
  string | null,
])[] = [
  ['pi-garage', '192.168.1.40', 'LINUX', 'raspbian', 'DOWN', 0.924, null, ['SSH'], 'HTTP check on :8123 refused the connection'],
  ['pop-os', '100.64.0.3', 'LINUX', 'pop', 'DEGRADED', 1, [3, 28, 82, 9], ['SSH', 'VNC', 'Wake'], "Grafana's TLS certificate expires in 9 days"],
  ['nas', 'nas.lan', 'LINUX', 'debian', 'DEGRADED', 1, [8, 34, 91], ['SSH'], 'Responses are slower than 500 ms'],
  ['win-gaming', 'win.lan', 'WINDOWS', null, 'UNKNOWN', null, null, ['RDP', 'Wake'], null],
  ['atlas', 'atlas.lan', 'LINUX', 'ubuntu', 'UP', 0.999, [23, 61, 78, 34], ['SSH', 'RDP', 'Wake'], null],
  ['edge-vps', '203.0.113.24', 'LINUX', 'debian', 'UP', 1, [3, 22, 40], ['SSH'], null],
  ['k3s-a', '10.0.20.11', 'LINUX', 'ubuntu', 'UP', 1, [46, 70, 52], ['SSH'], null],
  ['k3s-b', '10.0.20.12', 'LINUX', 'ubuntu', 'UP', 0.998, [38, 66, 51], ['SSH'], null],
  ['pve', 'pve.lan', 'LINUX', 'debian', 'UP', 1, [17, 58, 63], ['SSH', 'Wake'], null],
  ['router', '192.168.1.1', 'LINUX', 'openwrt', 'UP', 1, [2, 31, 12], ['SSH'], null],
  ['studio-mac', 'studio.lan', 'MACOS', null, 'UP', 0.992, [41, 72, 55], ['SSH', 'VNC', 'Wake'], null],
  ['workbench', 'workbench.lan', 'LINUX', 'fedora', 'UP', 0.971, [12, 44, 35], ['SSH', 'VNC'], null],
];
