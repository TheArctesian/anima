import type { ContainerDetail } from '../api';

// HOST-017: the details of one container as rows of a label and a text. The hub gives the names of the environment
// variables and never a value.

export interface DetailRow {
  readonly label: string;
  readonly value: string;
  /** Machine text: a command, a path, an address. */
  readonly mono?: boolean;
}

function time(at: string | null | undefined): string {
  if (at == null) return 'Never';
  const instant = new Date(at);
  return Number.isNaN(instant.getTime()) ? 'Never' : instant.toLocaleString();
}

/** The facts of the container itself. */
export function summaryRows(detail: ContainerDetail): readonly DetailRow[] {
  return [
    { label: 'Image', value: detail.image, mono: true },
    { label: 'Command', value: detail.command.join(' '), mono: true },
    { label: 'Working directory', value: detail.workingDir === '' ? 'The default of the image' : detail.workingDir, mono: detail.workingDir !== '' },
    { label: 'User', value: detail.user === '' ? 'The default of the image' : detail.user },
    { label: 'Restart policy', value: detail.restartPolicy },
    { label: 'Restarts', value: String(detail.restartCount) },
    { label: 'Started', value: time(detail.startedAt) },
    { label: 'Finished', value: time(detail.finishedAt) },
    { label: 'Exit status', value: String(detail.exitCode) },
  ];
}

const HEALTH_WORDS: Readonly<Record<ContainerDetail['health'], string>> = { HEALTHY: 'Healthy', UNHEALTHY: 'Unhealthy', STARTING: 'Starting', NONE: 'Not running' };

/** The health check and its last result; empty when the container has no health check. */
export function healthRows(detail: ContainerDetail): readonly DetailRow[] {
  const check = detail.healthCheck;
  if (check == null) return [];
  const run = detail.lastHealthRun;
  return [
    { label: 'Test', value: check.test.join(' '), mono: true },
    { label: 'Interval', value: check.intervalSeconds == null ? 'The default of Docker (30 s)' : `${String(check.intervalSeconds)} s` },
    { label: 'Health', value: HEALTH_WORDS[detail.health] },
    { label: 'Failed in a row', value: String(detail.failingStreak) },
    { label: 'Last run', value: run == null ? 'None' : `Exit status ${String(run.exitCode)}, ${time(run.endedAt)}` },
  ];
}

/** One mount as "volume app-data → /data (read-write)". */
export function mountLine(mount: ContainerDetail['mounts'][number]): string {
  // A tmpfs mount has no volume and no path on the device.
  const from = [mount.type, mount.name ?? mount.source].filter((part) => part !== '').join(' ');
  return `${from} → ${mount.destination} (${mount.readOnly ? 'read-only' : 'read-write'})`;
}

/** One network as its name and the addresses of the container on it. */
export function networkLine(address: ContainerDetail['networks'][number]): string {
  const addresses = [address.ipv4, address.ipv6].filter((each): each is string => each != null);
  return addresses.length === 0 ? address.network : `${address.network}  ${addresses.join('  ')}`;
}
