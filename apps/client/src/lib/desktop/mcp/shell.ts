import { invoke } from '@tauri-apps/api/core';
import type { McpChange, McpKind, McpPermissions, McpState, McpTool } from '$lib/mcp/permissions';

/** Why the shell did not read or save the permissions. The codes are the shell's (MCP-018). */
export type McpShellProblem = 'invalid_hub' | 'no_folder' | 'unreadable' | 'unwritable' | 'invalid_change' | 'failed';

const MESSAGES: Record<McpShellProblem, string> = {
  invalid_hub: 'The hub address is not valid.',
  no_folder: 'This computer has no configuration folder for Anima.',
  unreadable: 'The saved MCP permissions cannot be read. Fix or delete the configuration file, or run "anima mcp permissions reset".',
  unwritable: 'The permissions could not be saved to the configuration file.',
  invalid_change: 'The shell does not know that tool.',
  failed: 'The permissions could not be changed.',
};

const PROBLEMS = Object.keys(MESSAGES);

/** A failure of the shell. The message is fixed text; nothing from the shell's error is kept, so no path reaches the screen (LOG-007). */
export class McpShellError extends Error {
  constructor(readonly problem: McpShellProblem) {
    super(MESSAGES[problem]);
    this.name = 'McpShellError';
  }
}

/** What the section needs from the shell; the store takes it as a parameter, so tests pass their own. */
export interface McpShell {
  read(hub: string): Promise<McpPermissions>;
  write(hub: string, change: McpChange): Promise<McpPermissions>;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function isState(value: unknown): value is McpState {
  return value === 'allow' || value === 'ask' || value === 'off';
}

function isKind(value: unknown): value is McpKind {
  return value === 'read' || value === 'change';
}

function toolFrom(row: unknown): McpTool | null {
  if (!isRecord(row)) return null;
  const { toolset, tool, kind, needsSignIn, state, description } = row;
  if (typeof toolset !== 'string' || typeof tool !== 'string' || typeof description !== 'string') return null;
  if (!isKind(kind) || !isState(state) || typeof needsSignIn !== 'boolean') return null;
  return { toolset, tool, kind, needsSignIn, state, description };
}

/** The shell's answer as permissions, or `null` when it is not one. The answer crosses a process boundary, so every field is checked. */
export function permissionsFrom(answer: unknown): McpPermissions | null {
  if (!isRecord(answer) || typeof answer.hub !== 'string' || typeof answer.summary !== 'string' || !Array.isArray(answer.tools)) return null;
  const tools = (answer.tools as unknown[]).map(toolFrom);
  if (tools.some((tool) => tool === null)) return null;
  return { hub: answer.hub, summary: answer.summary, tools: tools as McpTool[] };
}

function problemOf(reason: unknown): McpShellProblem {
  const code = isRecord(reason) ? reason.code : undefined;
  return typeof code === 'string' && PROBLEMS.includes(code) ? (code as McpShellProblem) : 'failed';
}

async function ask(command: string, args: Record<string, unknown>): Promise<McpPermissions> {
  let answer: unknown;
  try {
    answer = await invoke(command, args);
  } catch (reason) {
    throw new McpShellError(problemOf(reason));
  }
  const permissions = permissionsFrom(answer);
  if (permissions === null) throw new McpShellError('failed');
  return permissions;
}

/** The desktop shell's permission commands: the same code and file as `anima mcp permissions` (MCP-018). */
export const desktopMcpShell: McpShell = {
  read: (hub) => ask('mcp_permissions_read', { hubOrigin: hub }),
  write: (hub, change) => ask('mcp_permissions_write', { hubOrigin: hub, change }),
};
