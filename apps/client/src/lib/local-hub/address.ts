import type { LocalHubStatus } from '$lib/desktop/local-hub/shell';

/** The origin of this computer's own hub, or `null` when it is not installed and ready (AWK-005, FRN-008). */
export function localHubOrigin(status: Pick<LocalHubStatus, 'installed' | 'state' | 'hub'>): string | null {
  if (!status.installed || status.state !== 'ready' || status.hub === undefined || status.hub === '') return null;
  return `http://${status.hub}`;
}
