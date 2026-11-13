/**
 * What this client can do: one yes/no answer per desktop-only feature (WP-42 D-4). Every desktop-only control reads
 * these answers through `$lib/platform/platform`, so the web build turns the features off in one place (CLI-001).
 */
export interface Capabilities {
  /** The OS key store holds the access token (CLI-003). */
  readonly keyStore: boolean;
  /** Web interfaces open in native web view tabs that can pin a certificate (WEB-003, WEB-008). */
  readonly webViewTabs: boolean;
  /** A session tab can move to its own window (RA-002). */
  readonly popOut: boolean;
  /** The app checks for and installs its own updates (UPD-001, UPD-006). */
  readonly updates: boolean;
  /** The app keeps a process log and can open its folder (LOG-008). */
  readonly processLog: boolean;
  /** The shell can read the computer the app runs on, for Add device (R-058, R-060). */
  readonly thisMachine: boolean;
  /** The shell can open System Settings at Sharing, for Remote Login and Screen Sharing (R-059, R-079). */
  readonly systemSettings: boolean;
  /** This computer can build releases: the shell runs the build steps and keeps the builder token (BLD-020, BLD-026). */
  readonly builds: boolean;
  /** The shell reads and saves the `anima mcp` permissions in the command line's configuration file (MCP-018, MCP-023). */
  readonly mcpPermissions: boolean;
  /** The shell puts the `anima` program the app carries on the `PATH` with a link or launcher (INS-004 to INS-006). */
  readonly commandLine: boolean;
  /** The window draws its content under the OS title bar, with the window buttons over the navigation bar (macOS). */
  readonly overlayTitleBar: boolean;
}
