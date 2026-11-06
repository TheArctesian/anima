import type { DeviceCheck } from '$lib/checks/api';
import type { CheckType } from '$lib/hub/generated/graphql';

const TYPE_LABELS: Record<CheckType, string> = {
  ICMP: 'Ping',
  TCP: 'TCP',
  TLS: 'TLS',
  HTTP: 'HTTP',
  DNS: 'DNS',
  PUSH: 'Push',
  GROUP: 'Group',
  DOCKER: 'Docker',
  SNMP: 'SNMP',
  TAILSCALE_PING: 'Tailscale ping',
  SYSTEMD: 'systemd',
};

const NONE = '—';

/** What a check row shows besides its name, status and pause toggle (D-10). */
export interface CheckRowText {
  readonly type: string;
  readonly responseTime: string;
  /** CHK-007: days until the certificate expires; null for a check that reads no certificate. */
  readonly certificate: string | null;
}

/** A response time in milliseconds as text, "—" for none. */
export function msText(ms: number | null | undefined): string {
  return ms == null ? NONE : `${String(ms)} ms`;
}

export function checkRowText(check: Pick<DeviceCheck, 'type' | 'lastHeartbeat' | 'certificateDaysLeft'>): CheckRowText {
  const days = check.certificateDaysLeft;
  return {
    type: TYPE_LABELS[check.type],
    responseTime: msText(check.lastHeartbeat?.responseTimeMs),
    certificate: days == null ? null : `Certificate: ${String(days)} ${Math.abs(days) === 1 ? 'day' : 'days'}`,
  };
}
