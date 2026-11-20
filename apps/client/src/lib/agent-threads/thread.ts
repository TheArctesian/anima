/** The threads of this computer (THR-011 to THR-016): what the desktop shell answers, checked, and how the page orders it. */

export type Program = 'claude_code' | 'codex';

/** One row of the list: a session of an agent program, read from its file. */
export interface ThreadRow {
  readonly id: string;
  readonly program: Program;
  readonly title: string | null;
  readonly folder: string | null;
  readonly branch: string | null;
  readonly model: string | null;
  readonly started: Date | null;
  readonly last: Date | null;
}

export type ToolKind = 'command' | 'edit' | 'read' | 'search' | 'web' | 'agent' | 'plan' | 'question' | 'other';

export interface ToolCall {
  readonly name: string;
  readonly kind: ToolKind;
  readonly summary: string;
  readonly input: string;
  readonly patch: string | null;
  readonly output: string | null;
  readonly failed: boolean;
}

export type Entry =
  | { readonly kind: 'prompt'; readonly at: Date | null; readonly text: string; readonly images: number }
  | { readonly kind: 'reply' | 'reasoning'; readonly at: Date | null; readonly text: string }
  | { readonly kind: 'tool'; readonly at: Date | null; readonly tool: ToolCall }
  | { readonly kind: 'turn_end'; readonly at: Date | null; readonly stopped: boolean }
  | { readonly kind: 'compacted'; readonly at: Date | null };

export interface Thread extends ThreadRow {
  readonly prompts: number;
  readonly entries: readonly Entry[];
  /** Entries past the limit of a thread, not shown. */
  readonly dropped: number;
}

/** What the page needs from the desktop shell; tests pass their own. */
export interface ThreadsShell {
  list(program: Program | null): Promise<readonly ThreadRow[]>;
  read(id: string): Promise<Thread>;
}

const PROGRAMS: readonly string[] = ['claude_code', 'codex'];
const TOOL_KINDS: readonly string[] = ['command', 'edit', 'read', 'search', 'web', 'agent', 'plan', 'question', 'other'];

export function programName(program: Program): string {
  return program === 'claude_code' ? 'Claude Code' : 'Codex';
}

/** The last part of a folder path: the name a person knows the folder by. */
export function folderName(folder: string | null): string {
  if (folder === null) return 'No folder';
  const parts = folder.split('/').filter((part) => part !== '');
  return parts.at(-1) ?? folder;
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function text(value: unknown): string | null {
  return typeof value === 'string' && value !== '' ? value : null;
}

function time(value: unknown): Date | null {
  if (typeof value !== 'string') return null;
  const at = new Date(value);
  return Number.isNaN(at.getTime()) ? null : at;
}

function rowFrom(value: unknown): ThreadRow | null {
  if (!isRecord(value) || typeof value.id !== 'string' || value.id === '') return null;
  if (typeof value.program !== 'string' || !PROGRAMS.includes(value.program)) return null;
  return {
    id: value.id,
    program: value.program as Program,
    title: text(value.title),
    folder: text(value.folder),
    branch: text(value.branch),
    model: text(value.model),
    started: time(value.started),
    last: time(value.last),
  };
}

/** The shell's answer as rows, or `null` when it is not one. The answer crosses a process boundary, so each field is checked. */
export function rowsFrom(answer: unknown): readonly ThreadRow[] | null {
  if (!Array.isArray(answer)) return null;
  const rows = (answer as unknown[]).map(rowFrom);
  return rows.some((row) => row === null) ? null : (rows as ThreadRow[]);
}

function toolFrom(value: unknown): ToolCall | null {
  if (!isRecord(value) || typeof value.name !== 'string' || typeof value.summary !== 'string') return null;
  if (typeof value.kind !== 'string' || !TOOL_KINDS.includes(value.kind)) return null;
  return {
    name: value.name,
    kind: value.kind as ToolKind,
    summary: value.summary,
    input: typeof value.input === 'string' ? value.input : '',
    patch: text(value.patch),
    output: text(value.output),
    failed: value.failed === true,
  };
}

function entryFrom(value: unknown): Entry | null {
  if (!isRecord(value)) return null;
  const at = time(value.at);
  const body = typeof value.text === 'string' ? value.text : '';
  switch (value.kind) {
    case 'prompt':
      return { kind: 'prompt', at, text: body, images: typeof value.images === 'number' ? value.images : 0 };
    case 'reply':
    case 'reasoning':
      return { kind: value.kind, at, text: body };
    case 'tool': {
      const tool = toolFrom(value.tool);
      return tool === null ? null : { kind: 'tool', at, tool };
    }
    case 'turn_end':
      return { kind: 'turn_end', at, stopped: value.stopped === true };
    case 'compacted':
      return { kind: 'compacted', at };
    default:
      return null;
  }
}

/** The shell's answer as a thread, or `null` when it is not one. An entry of a kind this client does not know is left out. */
export function threadFrom(answer: unknown): Thread | null {
  const row = rowFrom(answer);
  if (row === null || !isRecord(answer) || !Array.isArray(answer.entries)) return null;
  const entries = (answer.entries as unknown[]).map(entryFrom).filter((entry) => entry !== null);
  return {
    ...row,
    prompts: typeof answer.prompts === 'number' ? answer.prompts : 0,
    entries,
    dropped: typeof answer.dropped === 'number' ? answer.dropped : 0,
  };
}

/** A file that changed in this time is marked as changed now. It is not a state of the agent (THR-013). */
export const CHANGED_NOW_MS = 2 * 60_000;

export type GroupName = 'Changed now' | 'Today' | 'Earlier';

export interface ThreadGroup {
  readonly name: GroupName;
  readonly rows: readonly ThreadRow[];
}

/** Whether the file of `row` changed in the last 2 minutes before `now`. */
export function changedNow(row: ThreadRow, now: Date): boolean {
  return row.last !== null && now.getTime() - row.last.getTime() < CHANGED_NOW_MS;
}

/** The rows in the three groups of the list, each in the order given; a group with no row is left out (THR-013). */
export function groups(rows: readonly ThreadRow[], now: Date): readonly ThreadGroup[] {
  const today = new Date(now.getFullYear(), now.getMonth(), now.getDate()).getTime();
  const named = (row: ThreadRow): GroupName => {
    if (changedNow(row, now)) return 'Changed now';
    return row.last !== null && row.last.getTime() >= today ? 'Today' : 'Earlier';
  };
  return (['Changed now', 'Today', 'Earlier'] as const)
    .map((name) => ({ name, rows: rows.filter((row) => named(row) === name) }))
    .filter((group) => group.rows.length > 0);
}

/** The rows of `program` (all when `null`) whose title, folder or branch holds `search`, without regard to case. */
export function narrowed(rows: readonly ThreadRow[], program: Program | null, search: string): readonly ThreadRow[] {
  const wanted = search.trim().toLowerCase();
  return rows.filter((row) => {
    if (program !== null && row.program !== program) return false;
    if (wanted === '') return true;
    return [row.title, row.folder, row.branch].some((each) => each?.toLowerCase().includes(wanted) === true);
  });
}

/** How long ago `last` was, in the short words of the list. */
export function ago(last: Date | null, now: Date): string {
  if (last === null) return '';
  const minutes = Math.floor((now.getTime() - last.getTime()) / 60_000);
  if (minutes < 1) return 'now';
  if (minutes < 60) return `${String(minutes)} min`;
  if (minutes < 24 * 60) return `${String(Math.floor(minutes / 60))} h`;
  return last.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
}

/** A part of the timeline: one entry, or the tool calls that follow each other, drawn as one block (THR-014). */
export type Part = { readonly kind: 'entry'; readonly entry: Exclude<Entry, { kind: 'tool' }> } | { readonly kind: 'tools'; readonly tools: readonly ToolCall[] };

/** The entries as parts: tool calls that follow each other are one block. */
export function parts(entries: readonly Entry[]): readonly Part[] {
  const out: Part[] = [];
  let run: ToolCall[] = [];
  const close = (): void => {
    if (run.length > 0) out.push({ kind: 'tools', tools: run });
    run = [];
  };
  for (const entry of entries) {
    if (entry.kind === 'tool') {
      run.push(entry.tool);
    } else {
      close();
      out.push({ kind: 'entry', entry });
    }
  }
  close();
  return out;
}

/** The words for the kind of a tool call. */
export function kindName(kind: ToolKind): string {
  const names: Record<ToolKind, string> = {
    command: 'Command',
    edit: 'Edit',
    read: 'Read',
    search: 'Search',
    web: 'Web',
    agent: 'Agent',
    plan: 'Plan',
    question: 'Question',
    other: 'Tool',
  };
  return names[kind];
}

/** The result of a call in a few words: the lines added and removed by an edit, a failure, or the lines of the output. */
export function resultWords(tool: ToolCall): string {
  if (tool.failed) return 'Failed';
  if (tool.patch !== null) {
    const lines = tool.patch.split('\n');
    const added = lines.filter((line) => line.startsWith('+') && !line.startsWith('+++')).length;
    const removed = lines.filter((line) => line.startsWith('-') && !line.startsWith('---')).length;
    return `+${String(added)} −${String(removed)}`;
  }
  if (tool.output === null) return '';
  const count = tool.output.split('\n').length;
  return count === 1 ? '1 line' : `${String(count)} lines`;
}
