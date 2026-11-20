import type { IndexEntry } from '$lib/page/contents/PageIndex.svelte';

/** What this platform can show: the Hub, Stay awake, Updates and MCP sections are desktop only (CLI-002, AWK-004, UPD-006, MCP-023). */
export interface IndexShown {
  readonly keyStore: boolean;
  readonly stayAwake: boolean;
  readonly desktopRows: boolean;
  readonly mcpPermissions: boolean;
  /** False on a hub that Anima.ing hosts: it has no VPN (R-222). */
  readonly vpn: boolean;
}

/** Each section of the page: its anchor, heading, icon and the line under the heading (SET-001). */
export const SECTIONS = {
  account: { id: 'settings-account', title: 'Account', icon: 'carbon:user-avatar', description: 'Who is signed in to this hub.' },
  leave: { id: 'settings-leave', title: 'Leave', icon: 'carbon:logout', description: 'Sign out and forget this hub.' },
  hub: { id: 'settings-hub', title: 'Hub', icon: 'carbon:network-enterprise', description: 'The hub this app talks to.' },
  // The next two show only when the shell or the hub says they apply, so the index leaves them out (FRN-006, FRN-009).
  localHub: { id: 'settings-local-hub', title: "This computer's hub", icon: 'carbon:bare-metal-server', description: 'The hub on this computer, its database and link.' },
  linkDevices: { id: 'settings-link-devices', title: 'Devices on the link', icon: 'carbon:remote-connection', description: 'Devices that reach this hub over the link.' },
  stayAwake: { id: 'settings-stay-awake', title: 'Stay awake', icon: 'carbon:power', description: 'Keeps this computer on for a hub or agent.' },
  appearance: { id: 'settings-appearance', title: 'Appearance', icon: 'carbon:color-palette', description: 'How Anima.ing looks on this device.' },
  notifications: { id: 'settings-notifications', title: 'Notifications', icon: 'carbon:notification', description: 'Where the hub sends alerts.' },
  checkDefaults: { id: 'settings-check-defaults', title: 'Default check settings', icon: 'carbon:settings-adjust', description: 'New checks start with these settings.' },
  retention: { id: 'settings-retention', title: 'Data retention', icon: 'carbon:data-backup', description: 'How long the hub keeps records.' },
  updates: { id: 'settings-updates', title: 'Updates', icon: 'carbon:update-now', description: "Version, command line tool, log and release builds." },
  apiKeys: { id: 'settings-api-keys', title: 'API keys', icon: 'carbon:api-key', description: "Keys for programs that call the hub's GraphQL API." },
  extensions: { id: 'settings-extensions', title: 'Extensions', icon: 'carbon:grid', description: 'Features beyond devices. Off by default; changes save at once.' },
  integrations: { id: 'settings-integrations', title: 'Integrations', icon: 'carbon:link', description: 'Where Anima.ing reads records from. Keys stay in the hub.' },
  mcp: { id: 'settings-mcp', title: 'MCP', icon: 'carbon:security', description: 'What coding agents may do through anima mcp on this computer.' },
  vpn: { id: 'settings-vpn', title: 'VPN', icon: 'carbon:connect', description: 'How the hub reaches each site, and when each VPN key expires.' },
} as const satisfies Record<string, IndexEntry & { readonly description: string }>;

/** The three groups of the page, in its order: the hub and the account, this computer, what the hub is connected to (R-224). */
export const GROUPS = { hub: 'Hub and account', computer: 'This computer', connections: 'Connections' } as const;

/** The index of the sections the page shows, in the page's order and in its groups (WP-50 F-10, R-224). */
export function indexOf(shown: IndexShown): readonly IndexEntry[] {
  const { account, hub, stayAwake, appearance, notifications, checkDefaults, retention, updates, apiKeys, extensions, integrations, mcp, vpn } = SECTIONS;
  const groups: readonly (readonly [string, readonly (IndexEntry | false)[]])[] = [
    [GROUPS.hub, [account, shown.keyStore && hub, notifications, checkDefaults, retention]],
    [GROUPS.computer, [shown.stayAwake && stayAwake, appearance, shown.desktopRows && updates, shown.mcpPermissions && mcp]],
    [GROUPS.connections, [integrations, apiKeys, extensions, shown.vpn && vpn]],
  ];
  return groups.flatMap(([group, entries]) => entries.filter((entry) => entry !== false).map((entry) => ({ ...entry, group })));
}
