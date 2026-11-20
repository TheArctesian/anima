/** The coding agents of this computer and their entry `anima`, as the Settings part shows them (MCP-039). */

/** What the entry of an agent is to this program (MCP-027). */
export type AgentState = 'not_installed' | 'not_set_up' | 'set_up' | 'different' | 'unreadable';

/** One thing a change touches, shown before it is done (MCP-029). */
export type AgentStep =
  | { readonly kind: 'write'; readonly file: string; readonly copy: string | null }
  | { readonly kind: 'run'; readonly program: string; readonly arguments: readonly string[] };

export interface AgentRow {
  readonly id: string;
  readonly name: string;
  readonly state: AgentState;
  readonly steps: readonly AgentStep[];
  /** Why the last change of this agent failed; `null` when it did not. */
  readonly problem: string | null;
}

const WORDS: Readonly<Record<AgentState, string>> = {
  not_installed: 'Not on this computer',
  not_set_up: 'Not set up',
  set_up: 'Set up',
  different: 'Starts a different anima',
  unreadable: 'Its file cannot be read',
};

/** The tone of the dot beside the words; the words always say the state (MAT-005). */
const TONES: Readonly<Record<AgentState, 'up' | 'warning' | 'down' | 'unknown'>> = {
  not_installed: 'unknown',
  not_set_up: 'unknown',
  set_up: 'up',
  different: 'warning',
  unreadable: 'down',
};

export function stateWords(state: AgentState): string {
  return WORDS[state];
}

export function stateTone(state: AgentState): 'up' | 'warning' | 'down' | 'unknown' {
  return TONES[state];
}

/** The agents that are on this computer: only these are rows of the part. */
export function present(rows: readonly AgentRow[]): readonly AgentRow[] {
  return rows.filter((row) => row.state !== 'not_installed');
}

/** What the button of a row does: set up, remove, or nothing where the file cannot be read (MCP-034). */
export function rowAction(state: AgentState): 'install' | 'uninstall' | null {
  if (state === 'not_set_up' || state === 'different') return 'install';
  return state === 'set_up' ? 'uninstall' : null;
}

/** The agents that "Set up all" changes (MCP-028). */
export function toSetUp(rows: readonly AgentRow[]): readonly AgentRow[] {
  return rows.filter((row) => rowAction(row.state) === 'install');
}

/** One step in words, with each path whole: the person must see what is touched (MCP-029). */
export function stepWords(step: AgentStep): string {
  if (step.kind === 'run') return `runs ${[step.program, ...step.arguments].join(' ')}`;
  return step.copy === null ? `writes ${step.file}` : `writes ${step.file} and keeps a copy, ${step.copy}`;
}

/** What a change of `row` touches, as one line for the list before the change. */
export function touched(row: AgentRow): string {
  const steps = row.steps.map(stepWords).join('; then ');
  return row.state === 'different' ? `${steps}. This replaces your entry "anima".` : `${steps}.`;
}
