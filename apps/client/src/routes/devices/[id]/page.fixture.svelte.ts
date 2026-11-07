import type { DeviceCheck, NewHeartbeat } from '$lib/checks/api';
import type { DeviceDetail } from '$lib/devices/api';
import type { LogEvent } from '$lib/events/api';
import type { DeviceSession } from './sessions/sessions';

export const ID = '01890a5d-ac96-774b-bcce-b302099a8057';
export const OTHER_ID = '01890a5d-ac96-774b-bcce-b302099a8058';

export const KEY = { algorithm: 'ssh-ed25519', fingerprintSha256: 'SHA256:abc', firstSeenAt: '2026-11-04T09:00:00Z' };

export function device(change: Partial<DeviceDetail> = {}, ssh: Partial<DeviceDetail['ssh']> = {}): DeviceDetail {
  return {
    id: ID,
    name: 'nas',
    host: 'nas.home.arpa',
    ip: '192.168.1.20',
    kind: 'server',
    os: 'LINUX',
    tags: ['lab', 'storage'],
    maintenance: false,
    status: 'UP',
    site: { id: 's1', name: 'Home' },
    ssh: { enabled: true, port: 22, username: 'root', auth: 'PASSWORD', credentialStored: true, hostKey: KEY, ...ssh },
    macAddress: null,
    metricsSource: 'NONE',
    accessWarnings: [],
    rdp: {
      enabled: false,
      port: 3389,
      username: '',
      domain: null,
      security: 'ANY',
      ignoreCertificate: false,
      resizeMethod: 'DISPLAY_UPDATE',
      audioOut: true,
      microphone: false,
      sharedDrive: false,
      credentialStored: false,
    },
    vnc: { enabled: false, port: 5900, username: null, credentialStored: false },
    transfer: { upload: true, download: true, copy: true, paste: true },
    day: 0.9995,
    week: 0.99,
    month: null,
    ...change,
  };
}

export function check(change: Partial<DeviceCheck> = {}): DeviceCheck {
  return {
    id: 'c1',
    parent: null,
    type: 'ICMP',
    name: 'ping',
    intervalSeconds: 60,
    retries: 3,
    retryIntervalSeconds: 20,
    degradedThresholdMs: 1000,
    invert: false,
    paused: false,
    status: 'UP',
    certificateDaysLeft: null,
    lastHeartbeat: { at: '2026-11-04T10:00:00Z', status: 'UP', responseTimeMs: 12, message: null },
    config: { __typename: 'IcmpConfig', timeoutMs: 1000, ipVersion: 'V4' },
    ...change,
  };
}

export function heartbeat(at: string, responseTimeMs: number | null, change: Partial<NewHeartbeat> = {}): NewHeartbeat {
  return { checkId: 'c1', deviceId: ID, heartbeat: { at, status: 'UP', responseTimeMs }, ...change };
}

export function beatsPage(ats: readonly string[], hasNextPage = false) {
  return {
    pageInfo: { hasNextPage, endCursor: hasNextPage ? 'cursor-1' : null },
    edges: ats.map((at) => ({ node: { at, status: 'UP' as const, responseTimeMs: 20, message: null } })),
  };
}

export function session(id: string, startedAt: string, change: Partial<DeviceSession> = {}): DeviceSession {
  return { id, protocol: 'SSH', operatorName: 'ada', startedAt, endedAt: null, endReason: null, ...change };
}

export function event(id: string, deviceId: string | null, message: string): LogEvent {
  return { id, at: '2026-11-04T10:00:00Z', severity: 'INFO', source: 'CHECK', deviceId, deviceName: 'nas', message };
}

/** The hub answers for a page of this device with one check. */
export function answers(overrides: Record<string, unknown> = {}): Record<string, unknown> {
  return {
    Device: { device: device() },
    DeviceChecks: { device: { id: ID, checks: [check()] } },
    ...overrides,
  };
}

/** What `$app/state` shows the page; state, so a test can move the URL as Back would. */
export const route = $state({ param: ID, search: '', state: {} as App.PageState });
