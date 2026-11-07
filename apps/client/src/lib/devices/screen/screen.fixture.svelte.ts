import type { CardDevice } from '../api';
import { DEFAULT_QUERY, type ListQuery } from '../query';

/** The query in the URL, as Svelte state for a mounted screen in a test. */
export const url: { query: ListQuery; writes: ListQuery[] } = $state({ query: DEFAULT_QUERY, writes: [] });

/** A device with every card field; `over` changes some. */
export function device(id: string, over: Partial<CardDevice> = {}): CardDevice {
  return {
    id,
    name: `Device ${id}`,
    host: `${id}.lan`,
    ip: null,
    os: 'OTHER',
    tags: [],
    maintenance: false,
    status: 'UP',
    uptime: 0.9995,
    macAddress: null,
    metricsSource: 'NONE',
    readings: null,
    accessWarnings: [],
    rdp: { enabled: false },
    vnc: { enabled: false, username: null },
    ssh: { enabled: false, hostKey: null },
    checks: [],
    webInterfaces: [],
    ...over,
  };
}

/** A `Devices` answer holding `devices`. */
export function devicesAnswer(devices: readonly CardDevice[], next: string | null = null) {
  return {
    devices: {
      totalCount: devices.length,
      pageInfo: { hasNextPage: next !== null, endCursor: next },
      edges: devices.map((node) => ({ node })),
    },
  };
}
