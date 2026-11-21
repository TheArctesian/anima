import { browserSessions, type BrowserSessions } from './access';
import { withAdditions } from './device-additions';
import type { Fixtures, Row } from './fixtures';
import { ticketBook, type TicketBook } from './tickets';

/** The synthetic hub's memory for one browser context. */
export interface HubState {
  readonly fixtures: Fixtures;
  readonly browserSessions: BrowserSessions;
  readonly tickets: TicketBook;
  /** Sessions `startSession` opened, newest first. */
  readonly started: Row[];
  readonly now: () => Date;
}

export function newHubState(fixtures: Fixtures, now: () => Date): HubState {
  return { fixtures, browserSessions: browserSessions(), tickets: ticketBook(), started: [], now };
}

const CHECK_CONFIG_TYPES: Record<string, string> = {
  icmp: 'IcmpConfig',
  tcp: 'TcpConfig',
  tls: 'TlsConfig',
  http: 'HttpConfig',
  dns: 'DnsConfig',
  push: 'PushConfig',
  group: 'GroupConfig',
  docker: 'DockerConfig',
  snmp: 'SnmpConfig',
  tailscalePing: 'TailscalePingConfig',
  systemd: 'SystemdConfig',
  domainExpiry: 'DomainExpiryConfig',
};

interface Page {
  readonly first?: number | null;
}

/** One page of a Relay-style connection; the fixtures are small, so there is never a next page. */
export function connection(nodes: readonly unknown[], page: Page = {}): Row {
  const shown = page.first == null ? nodes : nodes.slice(0, page.first);
  return {
    totalCount: nodes.length,
    edges: shown.map((node, index) => ({ cursor: `c${String(index)}`, node })),
    pageInfo: { hasNextPage: shown.length < nodes.length, endCursor: shown.length > 0 ? `c${String(shown.length - 1)}` : null },
  };
}

/** A fixture row may give `status` and `message`, for the pictures of a device that is not up; each other row is up. */
function heartbeats(row: Row = {}): Row[] {
  const at = (minute: number): string => `2026-11-06T10:0${String(minute)}:00Z`;
  const status = row.status ?? 'UP';
  return [2, 1, 0].map((minute) => ({ at: at(minute), status, responseTimeMs: status === 'UP' ? 12 + minute : null, message: row.message ?? null }));
}

export function checkView(state: HubState, row: Row): Row {
  const [key, config] = Object.entries((row.config ?? {}) as Row)[0] ?? ['icmp', {}];
  return {
    intervalSeconds: 60,
    retries: 0,
    retryIntervalSeconds: 60,
    degradedThresholdMs: 1000,
    invert: false,
    paused: false,
    ...row,
    config: { __typename: CHECK_CONFIG_TYPES[key] ?? 'IcmpConfig', ...(config as Row) },
    status: row.status ?? 'UP',
    lastHeartbeat: heartbeats(row)[0],
    heartbeats: (page: Page) => connection(heartbeats(row), page),
    uptime: () => row.uptime ?? 1,
    device: () => deviceOf(state, String(row.deviceId)),
  };
}

const CHANNEL_SETTINGS_TYPES: Record<string, string> = {
  WEBHOOK: 'WebhookSettings',
  NTFY: 'NtfySettings',
  EMAIL: 'EmailSettings',
  APPRISE: 'AppriseSettings',
};

/** A notification channel with its settings typed by kind and its latest delivery, if any (NOT-002). */
export function channelView(state: HubState, row: Row): Row {
  const latest = state.fixtures.deliveries
    .filter((found) => found.channelId === row.id)
    .reduce<Row | null>((last, each) => (last === null || String(each.at) > String(last.at) ? each : last), null);
  return {
    ...row,
    settings: { __typename: CHANNEL_SETTINGS_TYPES[String(row.kind)], ...(row.settings as Row) },
    lastDelivery: latest,
  };
}

function seriesOf(state: HubState, deviceId: string, range: string): Row[] {
  const entry = state.fixtures.metrics.find((found) => found.deviceId === deviceId);
  return ((entry?.series ?? []) as Row[]).filter((found) => found.range === range);
}

const READINGS: Row = { at: '2026-11-06T10:00:00Z', cpuPercent: 18.5, memoryPercent: 41.2, diskPercent: 63, vramPercent: null, vramGpu: null };

export function deviceView(state: HubState, source: Row): Row {
  const row = withAdditions(source);
  const id = String(row.id);
  const site = state.fixtures.sites.find((found) => found.id === row.siteId);
  const gpus = (row.gpus ?? []) as Row[];
  return {
    ...row,
    site: site ?? { id: row.siteId, name: 'site', subnet: '10.0.0.0/24' },
    status: row.status ?? 'UP',
    metricsStale: false,
    daemonVersion: row.daemonVersion ?? null,
    sessionLimit: row.sessionLimit ?? null,
    readings: row.metricsSource === 'SSH' ? { ...READINGS, vramGpu: gpus[0] ?? null } : null,
    gpus,
    checks: () => state.fixtures.checks.filter((found) => found.deviceId === id).map((found) => checkView(state, found)),
    metrics: ({ range }: { range: string }) => seriesOf(state, id, range),
    uptime: () => row.uptime ?? 1,
    createdAt: '2026-10-01T09:00:00Z',
    updatedAt: '2026-10-01T09:00:00Z',
  };
}

export function deviceOf(state: HubState, id: string): Row | null {
  const row = state.fixtures.devices.find((found) => found.id === id);
  return row === undefined ? null : deviceView(state, row);
}

export function allDevices(state: HubState): Row[] {
  return state.fixtures.devices.map((row) => deviceView(state, row));
}

type EventLine = readonly [severity: string, source: string, message: string];

function eventsOf(device: Row | undefined, lines: readonly EventLine[], first: number): Row[] {
  const at = (second: number): string => `2026-11-06T09:00:${String(second).padStart(2, '0')}Z`;
  return lines.map(([severity, source, message], index) => ({
    id: `0190a000-0000-7000-8020-${String(first + index).padStart(12, '0')}`,
    at: at(first + index - 1),
    severity,
    source,
    deviceId: device?.id ?? null,
    deviceName: device?.name ?? null,
    message,
  }));
}

/** A few events of the first device, so the logs panel has rows; every host is an example host (CON-003). */
export function eventRows(state: HubState): Row[] {
  return eventsOf(state.fixtures.devices[0], [
    ['INFO', 'CHECK', 'Check recovered'],
    ['WARNING', 'DEVICE', 'Device entered maintenance'],
    ['ERROR', 'ACCESS', 'Sign-in refused'],
  ], 1);
}

/**
 * The events of one device, as the hub answers `events(filter: { deviceId })` for the Logs tab of its page (LOG-009).
 * Another device than the first has two events of its own. They are left out of the whole log: there each device name
 * is a link, and the specs find each device's link once on the devices page.
 */
export function deviceEventRows(state: HubState, deviceId: string): Row[] {
  const index = state.fixtures.devices.findIndex((device) => device.id === deviceId);
  if (index === 0) return eventRows(state);
  if (index < 0) return [];
  return eventsOf(state.fixtures.devices[index], [
    ['WARNING', 'CHECK', 'Check went down'],
    ['INFO', 'CHECK', 'Check is up again'],
  ], 10 + index * 2);
}

export function sessionRows(state: HubState): Row[] {
  return [...state.started, ...state.fixtures.sessions];
}
