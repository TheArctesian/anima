/** What the shell says about the hub that runs on this computer. */
export interface LocalHubStatus {
  readonly installed: boolean;
  readonly running: boolean;
  readonly state?: string;
  /** `host:port`, for example `127.0.0.1:8443`. */
  readonly hub?: string;
}

/** The origin of this computer's own hub, or `null` when it is not installed and ready (AWK-005, FRN-008). */
export function localHubOrigin(status: LocalHubStatus): string | null {
  if (!status.installed || status.state !== 'ready' || status.hub === undefined || status.hub === '') return null;
  return `http://${status.hub}`;
}
