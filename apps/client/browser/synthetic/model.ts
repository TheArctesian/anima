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

function heartbeats(): Row[] {
  const at = (minute: number): string => `2026-11-06T10:0${String(minute)}:00Z`;
  return [2, 1, 0].map((minute) => ({ at: at(minute), status: 'UP', responseTimeMs: 12 + minute, message: null }));
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
    status: 'UP',
    lastHeartbeat: heartbeats()[0],
    heartbeats: (page: Page) => connection(heartbeats(), page),
    uptime: () => 1,
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
    status: 'UP',
    readings: row.metricsSource === 'SSH' ? { ...READINGS, vramGpu: gpus[0] ?? null } : null,
    gpus,
    checks: () => state.fixtures.checks.filter((found) => found.deviceId === id).map((found) => checkView(state, found)),
    metrics: ({ range }: { range: string }) => seriesOf(state, id, range),
    uptime: () => 1,
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

/** A few events so the logs panel has rows; every host is an example host (CON-003). */
export function eventRows(state: HubState): Row[] {
  const first = state.fixtures.devices[0];
  const at = (second: number): string => `2026-11-06T09:00:${String(second).padStart(2, '0')}Z`;
  const rows: [string, string, string][] = [
    ['INFO', 'CHECK', 'Check recovered'],
    ['WARNING', 'DEVICE', 'Device entered maintenance'],
    ['ERROR', 'ACCESS', 'Sign-in refused'],
  ];
  return rows.map(([severity, source, message], index) => ({
    id: `0190a000-0000-7000-8020-00000000000${String(index + 1)}`,
    at: at(index),
    severity,
    source,
    deviceId: first?.id ?? null,
    deviceName: first?.name ?? null,
    message,
  }));
}

export function sessionRows(state: HubState): Row[] {
  return [...state.started, ...state.fixtures.sessions];
}
