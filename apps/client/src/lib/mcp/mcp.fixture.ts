import { applyChange, type McpChange, type McpPermissions, type McpTool } from './permissions';
import { McpShellError, type McpShell, type McpShellProblem } from '$lib/desktop/mcp-shell';

// Test support: a few tools in the shell's shape, and a shell that keeps their states in memory.

function tool(toolset: string, name: string, kind: McpTool['kind'], needsSignIn = false): McpTool {
  return { toolset, tool: name, kind, needsSignIn, state: kind === 'read' ? 'allow' : 'off', description: `About ${name}.` };
}

export const FIXTURE_TOOLS: readonly McpTool[] = [
  tool('devices', 'anima_device_get', 'read'),
  tool('devices', 'anima_device_create', 'change'),
  tool('devices', 'anima_device_delete', 'change', true),
  tool('checks', 'anima_check_list', 'read'),
  tool('checks', 'anima_check_update', 'change'),
  tool('vpn', 'anima_vpn_status', 'read'),
];

export const HUB = 'https://hub.example.com';

function shown(tools: readonly McpTool[]): McpPermissions {
  return { hub: HUB, summary: '', tools };
}

export interface FakeShell extends McpShell {
  /** The changes the shell was asked to save, in order. */
  readonly writes: McpChange[];
  /** What the next reads and writes fail with; `null` for none. */
  readProblem: McpShellProblem | null;
  writeProblem: McpShellProblem | null;
  /** Holds each write until `release` is called, to look at the screen while it is on its way. */
  hold: boolean;
  release: () => void;
}

export function fakeShell(start: readonly McpTool[] = FIXTURE_TOOLS): FakeShell {
  let tools = start;
  let gate: (() => void) | undefined;
  const shell: FakeShell = {
    writes: [],
    readProblem: null,
    writeProblem: null,
    hold: false,
    release: () => gate?.(),
    read: () => (shell.readProblem === null ? Promise.resolve(shown(tools)) : Promise.reject(new McpShellError(shell.readProblem))),
    write: async (_hub, change) => {
      shell.writes.push(change);
      if (shell.hold) await new Promise<void>((resolve) => (gate = resolve));
      if (shell.writeProblem !== null) throw new McpShellError(shell.writeProblem);
      tools = applyChange(tools, change);
      return shown(tools);
    },
  };
  return shell;
}
