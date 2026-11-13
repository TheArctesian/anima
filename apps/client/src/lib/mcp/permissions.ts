/** What the operator may set for the `anima mcp` server, and the pure rules around it (MCP-018..MCP-023). */

/** What the server does with a tool: it may be called, it asks the person first, or it is off (MCP-012). */
export type McpState = 'allow' | 'ask' | 'off';

/** A read tool reads from the hub; a change tool changes something (MCP-002). */
export type McpKind = 'read' | 'change';

/** One row of `anima mcp permissions show --json`. */
export interface McpTool {
  readonly toolset: string;
  /** `anima_<noun>_<verb>` (MCP-011). */
  readonly tool: string;
  readonly kind: McpKind;
  /** Only a sign-in token may call it; an API key never lists it (MCP-003). */
  readonly needsSignIn: boolean;
  readonly state: McpState;
  readonly description: string;
}

/** What `permissions show --json` prints, for one hub. */
export interface McpPermissions {
  readonly hub: string;
  readonly summary: string;
  readonly tools: readonly McpTool[];
}

/** One change to the saved states; the shell takes it as it is (MCP-018). */
export type McpChange =
  | { readonly type: 'choose'; readonly names: readonly string[]; readonly state: McpState }
  | { readonly type: 'bulk'; readonly toolset?: string; readonly kind?: McpKind; readonly state: McpState }
  | { readonly type: 'reset' };

/** The tool sets in the order the command line lists them. */
const TOOLSET_NAMES: Readonly<Record<string, string>> = {
  devices: 'Devices',
  checks: 'Checks',
  maintenance: 'Maintenance',
  events: 'Events',
  sessions: 'Sessions',
  notifications: 'Notifications',
  web: 'Web interfaces',
  releases: 'Releases',
  vpn: 'VPN',
  retention: 'Data retention',
};

export const STATES: readonly { readonly value: McpState; readonly label: string }[] = [
  { value: 'allow', label: 'Allow' },
  { value: 'ask', label: 'Ask' },
  { value: 'off', label: 'Off' },
];

export type StateCounts = Readonly<Record<McpState, number>>;

export interface ToolGroup {
  readonly toolset: string;
  readonly label: string;
  readonly tools: readonly McpTool[];
  readonly counts: StateCounts;
}

/** How many of `tools` are in each state. */
export function countStates(tools: readonly McpTool[]): StateCounts {
  const counts = { allow: 0, ask: 0, off: 0 };
  for (const { state } of tools) counts[state] += 1;
  return counts;
}

/** The tools by set, sets in the command line's order, then any set this client does not know, in the order met. */
export function groupTools(tools: readonly McpTool[]): readonly ToolGroup[] {
  const names = [...Object.keys(TOOLSET_NAMES), ...tools.map((tool) => tool.toolset)];
  return [...new Set(names)]
    .map((toolset) => {
      const own = tools.filter((tool) => tool.toolset === toolset);
      return { toolset, label: toolsetLabel(toolset), tools: own, counts: countStates(own) };
    })
    .filter((group) => group.tools.length > 0);
}

export function toolsetLabel(toolset: string): string {
  return TOOLSET_NAMES[toolset] ?? toolset;
}

/** `anima_check_update` as "Check update": the id without its prefix, in words. */
export function toolLabel(tool: string): string {
  const words = tool.replace(/^anima_/, '').replaceAll('_', ' ');
  return words.charAt(0).toUpperCase() + words.slice(1);
}

/** The state a tool has when nothing is saved: read tools allow, change tools off (MCP-002). */
function builtIn(kind: McpKind): McpState {
  return kind === 'read' ? 'allow' : 'off';
}

/** The tools after `change`, as the shell will answer; used to show a change before it is saved. */
export function applyChange(tools: readonly McpTool[], change: McpChange): readonly McpTool[] {
  return tools.map((tool) => {
    switch (change.type) {
      case 'reset':
        return { ...tool, state: builtIn(tool.kind) };
      case 'choose':
        return change.names.includes(tool.tool) ? { ...tool, state: change.state } : tool;
      case 'bulk':
        return inBulk(tool, change) ? { ...tool, state: change.state } : tool;
    }
  });
}

type Bulk = Extract<McpChange, { type: 'bulk' }>;

/** Whether a bulk change covers the tool: its set and its kind, when the change names them. */
function inBulk(tool: McpTool, change: Bulk): boolean {
  return (change.toolset === undefined || change.toolset === tool.toolset) && (change.kind === undefined || change.kind === tool.kind);
}

/** Whether the change turns change tools on without a question per call: the operator confirms it first (MCP-020). */
export function needsConfirmation(change: McpChange): boolean {
  return change.type === 'bulk' && change.state === 'allow' && change.kind !== 'read';
}

/** What a change did, for the quiet "Saved" line and the screen reader: "Devices: every tool set to ask (7 tools)." */
export function describeChange(change: McpChange, tools: readonly McpTool[]): string {
  if (change.type === 'reset') return 'Every tool is back to its default: read tools allow, change tools off.';
  if (change.type === 'choose') {
    const [only] = change.names;
    const name = only === undefined || change.names.length > 1 ? `${String(change.names.length)} tools` : toolLabel(only);
    return `${name}: ${change.state}.`;
  }
  const scope = change.toolset === undefined ? 'Every' : `${toolsetLabel(change.toolset)}: every`;
  const kind = change.kind === undefined ? '' : ` ${change.kind}`;
  const count = tools.filter((tool) => inBulk(tool, change)).length;
  return `${scope}${kind} tool set to ${change.state} (${String(count)} ${count === 1 ? 'tool' : 'tools'}).`;
}

export interface BulkAction {
  readonly label: string;
  readonly change: McpChange;
}

/** The bulk actions of the whole list, or of one tool set (which has no Reset: that forgets every saved state). */
export function bulkActions(toolset?: string): readonly BulkAction[] {
  const scope = toolset === undefined ? {} : { toolset };
  const actions: BulkAction[] = [
    { label: 'Allow all', change: { type: 'bulk', ...scope, state: 'allow' } },
    { label: 'Ask for all', change: { type: 'bulk', ...scope, state: 'ask' } },
    { label: toolset === undefined ? 'Turn off all changes' : 'Turn off changes', change: { type: 'bulk', ...scope, kind: 'change', state: 'off' } },
  ];
  return toolset === undefined ? [...actions, { label: 'Reset', change: { type: 'reset' } }] : actions;
}

/** "3 allow, 1 ask, 2 off": the counts in words, so no state rests on colour (UI-003). */
export function countsText(counts: StateCounts): string {
  return STATES.map(({ value }) => `${String(counts[value])} ${value}`).join(', ');
}
