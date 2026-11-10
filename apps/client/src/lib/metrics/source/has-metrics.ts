import type { MetricsSource } from '$lib/hub/generated/graphql';

/** True when the hub holds readings for a device with this source: SSH polling or the agent's pushes (MET-004, MET-011). */
export function hasMetrics(source: MetricsSource | null | undefined): boolean {
  return source === 'SSH' || source === 'AGENT';
}
