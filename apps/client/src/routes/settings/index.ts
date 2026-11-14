import type { IndexEntry } from '$lib/page/contents/PageIndex.svelte';

/** What this platform can show: the Hub, Updates and MCP sections are desktop only (CLI-002, UPD-006, MCP-023). */
export interface IndexShown {
  readonly keyStore: boolean;
  readonly desktopRows: boolean;
  readonly mcpPermissions: boolean;
}

/** Each section of the page: its anchor, heading, icon and the line under the heading (SET-001). */
export const SECTIONS = {
  account: { id: 'settings-account', title: 'Account', icon: 'carbon:user-avatar', description: 'Who is signed in to this hub.' },
  hub: { id: 'settings-hub', title: 'Hub', icon: 'carbon:network-enterprise', description: 'The hub this app talks to.' },
  appearance: { id: 'settings-appearance', title: 'Appearance', icon: 'carbon:color-palette', description: 'How Anima looks on this device.' },
  notifications: { id: 'settings-notifications', title: 'Notifications', icon: 'carbon:notification', description: 'Where the hub sends alerts.' },
  checkDefaults: { id: 'settings-check-defaults', title: 'Default check settings', icon: 'carbon:settings-adjust', description: 'New checks start with these settings.' },
  retention: { id: 'settings-retention', title: 'Data retention', icon: 'carbon:data-backup', description: 'How long the hub keeps each kind of record.' },
  updates: { id: 'settings-updates', title: 'Updates', icon: 'carbon:update-now', description: "This app's version, its command line tool, its log and its release builds." },
  apiKeys: { id: 'settings-api-keys', title: 'API keys', icon: 'carbon:api-key', description: "Keys for programs that call the hub's GraphQL API." },
  extensions: { id: 'settings-extensions', title: 'Extensions', icon: 'carbon:grid', description: 'What Anima does besides devices. Each starts off; a change is saved at once.' },
  integrations: { id: 'settings-integrations', title: 'Integrations', icon: 'carbon:link', description: 'The places Anima reads records from. Each account keeps its key in the hub.' },
  mcp: { id: 'settings-mcp', title: 'MCP', icon: 'carbon:security', description: 'What coding agents may do through the anima mcp server on this computer.' },
  vpn: { id: 'settings-vpn', title: 'VPN', icon: 'carbon:connect', description: 'How the hub reaches each site, and when each VPN key expires.' },
} as const satisfies Record<string, IndexEntry & { readonly description: string }>;

/** The index of the sections the page shows, in the page's order (WP-50 F-10). */
export function indexOf(shown: IndexShown): readonly IndexEntry[] {
  const { account, hub, appearance, notifications, checkDefaults, retention, updates, apiKeys, extensions, integrations, mcp, vpn } = SECTIONS;
  const entries: (IndexEntry | false)[] = [
    account,
    shown.keyStore && hub,
    appearance,
    notifications,
    checkDefaults,
    retention,
    shown.desktopRows && updates,
    apiKeys,
    extensions,
    integrations,
    shown.mcpPermissions && mcp,
    vpn,
  ];
  return entries.filter((entry) => entry !== false);
}
