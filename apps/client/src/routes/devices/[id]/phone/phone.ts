import type { DeviceCheck } from '$lib/checks/api';
import type { Beat } from '$lib/checks/heartbeats';
import { statusLabel } from '$lib/devices/card/card';

const NONE = '—';

/** The device's last response time, from its newest heartbeat that has one (mockup 5); "—" when none has. */
export function responseText(beats: readonly Beat[]): string {
  const timed = beats.findLast((beat) => beat.responseTimeMs != null);
  return timed?.responseTimeMs == null ? NONE : `${String(timed.responseTimeMs)} ms`;
}

/**
 * The value at the end of a check's row in the phone layout (mockup 5): the days a certificate or a domain has left,
 * else the last response time, else the check's status in words.
 */
export function checkValue(check: Pick<DeviceCheck, 'status' | 'certificateDaysLeft' | 'domainDaysLeft' | 'lastHeartbeat'>): string {
  const days = check.certificateDaysLeft ?? check.domainDaysLeft;
  if (days != null) return `${String(days)} d`;
  const ms = check.lastHeartbeat?.responseTimeMs;
  return ms == null ? statusLabel(check.status) : `${String(ms)} ms`;
}
