// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { AgentsShellError, rowsFrom, type AgentsShell } from '$lib/desktop/mcp/agents-shell';
import { present, rowAction, stepWords, toSetUp, touched, type AgentRow } from './agents';
import CodingAgents from './CodingAgents.svelte';

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn() }));

const WRITE = { kind: 'write', file: '/home/ann/.cursor/mcp.json', copy: '/home/ann/.cursor/mcp.json.before-anima' } as const;
const RUN = { kind: 'run', program: '/usr/bin/codex', arguments: ['mcp', 'add', 'anima', '--', '/home/ann/.local/bin/anima', 'mcp'] } as const;
const ROWS: readonly AgentRow[] = [
  { id: 'cursor', name: 'Cursor', state: 'not_set_up', steps: [WRITE], problem: null },
  { id: 'codex', name: 'Codex CLI', state: 'set_up', steps: [RUN], problem: null },
  { id: 'zed', name: 'Zed', state: 'unreadable', steps: [], problem: null },
  { id: 'gemini', name: 'Gemini CLI', state: 'not_installed', steps: [], problem: null },
];

let app: ReturnType<typeof mount> | undefined;
afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

const settle = async (): Promise<void> => {
  for (let turn = 0; turn < 12; turn += 1) await Promise.resolve();
  flushSync();
};
const button = (label: string) => [...document.querySelectorAll<HTMLButtonElement>('button')].find((each) => (each.getAttribute('aria-label') ?? each.textContent.trim()) === label);

it('MCP-039: only the agents on this computer are rows, each with what its button does', () => {
  expect(present(ROWS).map((row) => [row.id, rowAction(row.state)])).toEqual([['cursor', 'install'], ['codex', 'uninstall'], ['zed', null]]);
  expect(toSetUp(ROWS).map((row) => row.id)).toEqual(['cursor']);
});

it('MCP-029: a step names each file with its copy, or the whole command line, and a replaced entry says so', () => {
  expect(stepWords(WRITE)).toBe('writes /home/ann/.cursor/mcp.json and keeps a copy, /home/ann/.cursor/mcp.json.before-anima');
  expect(stepWords(RUN)).toBe('runs /usr/bin/codex mcp add anima -- /home/ann/.local/bin/anima mcp');
  expect(touched({ ...ROWS[0], state: 'different' } as AgentRow)).toContain('This replaces your entry "anima".');
});

it('MCP-039: an answer of the shell that is not rows is refused', () => {
  expect(rowsFrom(ROWS)).toEqual(ROWS);
  expect(rowsFrom([{ ...ROWS[0], state: 'unknown' }])).toBeNull();
  expect(rowsFrom([{ ...ROWS[0], steps: [{ kind: 'write' }] }])).toBeNull();
  expect(rowsFrom({})).toBeNull();
});

it('MCP-029, MCP-039: a change is listed first and is made only after the person confirms', async () => {
  const done = ROWS.map((row) => (row.id === 'cursor' ? { ...row, state: 'set_up' as const } : row));
  const shell: AgentsShell = { read: vi.fn<AgentsShell['read']>().mockResolvedValue(ROWS), change: vi.fn<AgentsShell['change']>().mockResolvedValue(done) };
  app = mount(CodingAgents, { target: document.body, props: { shell } });
  await settle();
  expect([...document.querySelectorAll('.agents > ul > li .name')].map((each) => each.textContent)).toEqual(['Cursor', 'Codex CLI', 'Zed']);
  expect(document.body.textContent).toContain('This changes no permission');
  button('Set up all')?.click();
  await settle();
  expect(shell.change).not.toHaveBeenCalled();
  expect(document.querySelector('.asking')?.textContent).toContain('writes /home/ann/.cursor/mcp.json and keeps a copy');
  button('Cancel')?.click();
  await settle();
  expect(shell.change).not.toHaveBeenCalled();
  button('Set up Cursor')?.click();
  await settle();
  button('Set up')?.click();
  await settle();
  expect(shell.change).toHaveBeenCalledWith({ uninstall: false, agents: ['cursor'], force: false });
  expect(button('Remove from Cursor')).toBeDefined();
});

it('MCP-039: with no command line tool the part says to install it first, and shows no row', async () => {
  const shell: AgentsShell = { read: vi.fn<AgentsShell['read']>().mockRejectedValue(new AgentsShellError('no_command_line')), change: vi.fn<AgentsShell['change']>() };
  app = mount(CodingAgents, { target: document.body, props: { shell } });
  await settle();
  expect(document.querySelector('[role="alert"]')?.textContent).toContain('Install the command line tool first');
  expect(document.querySelector('.agents > ul > li')).toBeNull();
});
