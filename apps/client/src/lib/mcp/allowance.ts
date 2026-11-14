import { toolsetLabel, type McpChange } from './permissions';

/*
 * What an agent could do, without asking, once a tool set is on Allow. It comes from "What a compromised agent can do"
 * in docs/cli-mcp-plan.md: an agent steered by text it reads (a device name, a log line) uses what is on. `key` is what a
 * write API key reaches; `signIn` is what only the operator's own sign-in reaches (MCP-003). Sets with no change tool
 * have no entry. Phrases are short, so they read as lines in a dialog (MCP-020).
 */
interface Consequences {
  readonly key: readonly string[];
  readonly signIn: readonly string[];
}

const CONSEQUENCES: Readonly<Record<string, Consequences>> = {
  devices: { key: ['send wake packets'], signIn: ['delete devices with their stored credentials'] },
  checks: { key: ['pause or delete checks', 'point HTTP checks at internal URLs'], signIn: [] },
  maintenance: { key: ['start or delete maintenance windows'], signIn: [] },
  sessions: { key: ['end live sessions'], signIn: [] },
  notifications: { key: ['silence checks by emptying their notification channels', 'send test notifications'], signIn: [] },
  web: { key: ['repoint web interfaces at other addresses', 'delete web interfaces'], signIn: [] },
  releases: { key: [], signIn: ['start a release', 'cancel builds'] },
  vpn: { key: [], signIn: ['approve a VPN route'] },
  retention: { key: [], signIn: ['cut every data retention period to the minimum'] },
  extensions: { key: [], signIn: ['set extensions on or off'] },
  sources: { key: ['pause or start a sync of a connected account'], signIn: ['disconnect an account and delete its records'] },
  projects: { key: ['leave repositories out of Projects', 'confirm where a project runs'], signIn: [] },
};

/** The text of the question that comes before "Allow all", for its scope. */
export interface AllowQuestion {
  readonly title: string;
  readonly body: string;
  /** What an agent could then do without asking; at most three lines. */
  readonly lines: readonly string[];
  readonly confirmLabel: string;
}

/** What an agent could do through the sets given: the phrases of the data, in the order of the sets. */
export function consequencesOf(toolsets: readonly string[]): Consequences {
  const found = toolsets.map((toolset) => CONSEQUENCES[toolset]).filter((each) => each !== undefined);
  return { key: found.flatMap((each) => each.key), signIn: found.flatMap((each) => each.signIn) };
}

/** The question for a bulk "Allow all": its scope in the title and body, and only the consequences that belong to that scope. */
export function allowQuestion(change: Extract<McpChange, { type: 'bulk' }>, toolsets: readonly string[]): AllowQuestion {
  const scope = change.toolset === undefined ? 'every tool on this hub' : `every ${toolsetLabel(change.toolset)} tool`;
  const { key, signIn } = consequencesOf(change.toolset === undefined ? toolsets : [change.toolset]);
  const risky = key.length + signIn.length > 0;
  const lines = [
    key.length > 0 ? `With an API key that can write: ${key.join('; ')}.` : '',
    signIn.length > 0 ? `When anima mcp runs with your own sign-in, also: ${signIn.join('; ')}.` : '',
    risky ? 'The hub token still limits what an agent may do.' : '',
  ].filter((line) => line !== '');
  const effect = risky
    ? 'Text that an agent reads can steer it, so it could then do this without asking you:'
    : 'This scope has no tool that changes the hub.';
  return {
    title: `Allow ${scope}?`,
    body: `This turns on ${scope}, the read tools too, with no question for each call. ${effect}`,
    lines,
    confirmLabel: change.toolset === undefined ? 'Allow every tool' : `Allow every ${toolsetLabel(change.toolset)} tool`,
  };
}
