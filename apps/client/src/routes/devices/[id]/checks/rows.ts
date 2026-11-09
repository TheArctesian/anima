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
  DOMAIN_EXPIRY: 'Domain expiry',
};

const NONE = '—';

/** What a check row shows besides its name, status and pause toggle (D-10). */
export interface CheckRowText {
  readonly type: string;
  readonly responseTime: string;
  /**
   * CHK-007, D-29: days until the certificate or the domain registration expires;
   * null for a check that reads neither. One line, so the row keeps its height (MOT-007).
   */
  readonly expiry: string | null;
}

/** A response time in milliseconds as text, "—" for none. */
export function msText(ms: number | null | undefined): string {
  return ms == null ? NONE : `${String(ms)} ms`;
}

function daysText(label: string, days: number | null | undefined): string | null {
  return days == null ? null : `${label}: ${String(days)} ${Math.abs(days) === 1 ? 'day' : 'days'}`;
}

export function checkRowText(
  check: Pick<DeviceCheck, 'type' | 'lastHeartbeat' | 'certificateDaysLeft' | 'domainDaysLeft'>,
): CheckRowText {
  return {
    type: TYPE_LABELS[check.type],
    responseTime: msText(check.lastHeartbeat?.responseTimeMs),
    expiry:
      check.type === 'DOMAIN_EXPIRY'
        ? daysText('Domain', check.domainDaysLeft)
        : daysText('Certificate', check.certificateDaysLeft),
  };
}
