import { invoke } from '@tauri-apps/api/core';
import type { AgentRow, AgentState, AgentStep } from '$lib/mcp/agents/agents';

/** Why the shell did not read or change the agents. The codes are the shell's (MCP-039). */
export type AgentsShellProblem = 'caller' | 'no_command_line' | 'command' | 'user' | 'failed';

const MESSAGES: Record<AgentsShellProblem, string> = {
  caller: 'Only the main window can set up the coding agents.',
  no_command_line: 'Install the command line tool first: the agents start it. Settings, Updates, Command line tool.',
  command: 'The installed anima is in a folder that a different user can change. Install the command line tool again.',
  user: 'The home folder does not belong to the user that runs Anima.ing.',
  failed: 'The coding agents could not be read or changed.',
};

const PROBLEMS = Object.keys(MESSAGES);
const STATES: readonly string[] = ['not_installed', 'not_set_up', 'set_up', 'different', 'unreadable'];

/** A failure of the shell. The message is fixed text: nothing of the shell's error reaches the screen (LOG-007). */
export class AgentsShellError extends Error {
  constructor(readonly problem: AgentsShellProblem) {
    super(MESSAGES[problem]);
    this.name = 'AgentsShellError';
  }
}

/** What the part needs from the shell; tests pass their own. */
export interface AgentsShell {
  read(uninstall: boolean): Promise<readonly AgentRow[]>;
  change(wanted: { readonly uninstall: boolean; readonly agents: readonly string[]; readonly force: boolean }): Promise<readonly AgentRow[]>;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function stepFrom(value: unknown): AgentStep | null {
  if (!isRecord(value)) return null;
  if (value.kind === 'write' && typeof value.file === 'string' && (value.copy === null || typeof value.copy === 'string')) {
    return { kind: 'write', file: value.file, copy: value.copy };
  }
  if (value.kind === 'run' && typeof value.program === 'string' && Array.isArray(value.arguments) && value.arguments.every((each) => typeof each === 'string')) {
    return { kind: 'run', program: value.program, arguments: value.arguments as string[] };
  }
  return null;
}

function rowFrom(value: unknown): AgentRow | null {
  if (!isRecord(value) || typeof value.id !== 'string' || typeof value.name !== 'string') return null;
  if (typeof value.state !== 'string' || !STATES.includes(value.state) || !Array.isArray(value.steps)) return null;
  if (value.problem !== null && typeof value.problem !== 'string') return null;
  const steps = (value.steps as unknown[]).map(stepFrom);
  if (steps.some((step) => step === null)) return null;
  return { id: value.id, name: value.name, state: value.state as AgentState, steps: steps as AgentStep[], problem: value.problem };
}

/** The shell's answer as rows, or `null` when it is not one. The answer crosses a process boundary, so each field is checked. */
export function rowsFrom(answer: unknown): readonly AgentRow[] | null {
  if (!Array.isArray(answer)) return null;
  const rows = (answer as unknown[]).map(rowFrom);
  return rows.some((row) => row === null) ? null : (rows as AgentRow[]);
}

async function ask(command: string, args: Record<string, unknown>): Promise<readonly AgentRow[]> {
  let answer: unknown;
  try {
    answer = await invoke(command, args);
  } catch (reason) {
    const code = isRecord(reason) ? reason.code : undefined;
    throw new AgentsShellError(typeof code === 'string' && PROBLEMS.includes(code) ? (code as AgentsShellProblem) : 'failed');
  }
  const rows = rowsFrom(answer);
  if (rows === null) throw new AgentsShellError('failed');
  return rows;
}

/** The desktop shell's commands: the same library as `anima mcp install` (MCP-039). */
export const desktopAgentsShell: AgentsShell = {
  read: (uninstall) => ask('mcp_agents_read', { uninstall }),
  change: (wanted) => ask('mcp_agents_change', { wanted }),
};
