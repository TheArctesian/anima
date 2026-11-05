/** An update the feed offers: its version, and the download and install of it (UPD-003). */
export interface AvailableUpdate {
  readonly version: string;
  /** Downloads, verifies the signature (UPD-002) and installs. Rejects when any of that fails. */
  install(): Promise<void>;
}

/** What the update schedule needs from the desktop shell; `$lib/desktop/updater` implements it. */
export interface UpdaterPort {
  /** The update the feed offers, or `null` when the app is current. Rejects when the feed cannot be read. */
  check(): Promise<AvailableUpdate | null>;
  /** Restarts the app into the installed update (UPD-003: only when the operator asks). */
  restart(): Promise<void>;
  /** The version of the running app. */
  currentVersion(): Promise<string>;
}

/** Where the schedule records a failure; fixed text only, since the process log is a file on disk (LOG-007). */
export interface UpdateLog {
  warn(text: string): void;
}
