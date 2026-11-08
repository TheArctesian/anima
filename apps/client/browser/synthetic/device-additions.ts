import type { Row } from './fixtures';

// What the acceptance specs need on a device that `contracts/fixtures` does not give it. The fixtures stay as they are
// (K6); the synthetic hub adds these on the way out, as a hub whose operator had already set them up would answer.

const SSH_DEVICE = '0190a000-0000-7000-8012-000000000004';
const SINGLE_HTTP_DEVICE = '0190a000-0000-7000-8012-000000000003';
const SINGLE_FTP_DEVICE = '0190a000-0000-7000-8012-000000000002';

/** The fixtures hold no trusted SSH host key, so every SSH device would ask to "Test the SSH connection" first (K1 `HostKey`). */
const TRUSTED_HOST_KEY: Row = {
  algorithm: 'ssh-ed25519',
  fingerprintSha256: 'bGFiLWV4YW1wbGUtaG9zdC1rZXktbm90LXJlYWw',
  firstSeenAt: '2026-10-01T09:00:00Z',
};

function singleInterface(deviceId: string, label: string, url: string): Row {
  return { id: `0190a000-0000-7000-8015-${deviceId.slice(-12)}`, deviceId, label, url, openIn: 'ANIMA_TAB', pinnedFingerprintSha256: null, pinnedAt: null };
}

/** One unpinned `http` interface on the VNC device, and one `ftp` interface on win-legacy, whose scheme the browser must refuse (WEB-011). */
const WEB_INTERFACES: Record<string, Row[]> = {
  [SINGLE_HTTP_DEVICE]: [singleInterface(SINGLE_HTTP_DEVICE, 'Dashboard', 'http://mac-mini.annex.example:3000/status')],
  [SINGLE_FTP_DEVICE]: [singleInterface(SINGLE_FTP_DEVICE, 'Files', 'ftp://win-legacy.annex.example/share')],
};

export function withAdditions(row: Row): Row {
  const id = String(row.id);
  const ssh = row.ssh as Row | undefined;
  const trusted = ssh?.enabled === true && ssh.hostKey == null && id === SSH_DEVICE ? { ssh: { ...ssh, hostKey: TRUSTED_HOST_KEY } } : {};
  const webInterfaces = WEB_INTERFACES[id] === undefined ? {} : { webInterfaces: WEB_INTERFACES[id] };
  return { ...row, ...trusted, ...webInterfaces };
}
