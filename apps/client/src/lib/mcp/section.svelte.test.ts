// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import { fakeShell, HUB, type FakeShell } from './mcp.fixture';
import McpSection from './McpSection.svelte';

const mocks = vi.hoisted(() => ({ hub: { address: { origin: 'https://hub.example.com' } } }));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;
let shell: FakeShell;

beforeEach(() => {
  installAnimations();
  shell = fakeShell();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

async function settled(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

async function open(): Promise<HTMLElement> {
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(McpSection, { target, props: { shell } });
  flushSync();
  await settled();
  return target;
}

function button(name: string): HTMLButtonElement {
  const found = [...document.querySelectorAll('button')].find((each) => each.getAttribute('aria-label') === name || each.textContent.trim() === name);
  if (found === undefined) throw new Error(`no button ${name}`);
  return found;
}

async function click(element: HTMLElement): Promise<void> {
  element.click();
  await settled();
}

function radio(tool: string, state: string): HTMLButtonElement {
  const group = document.querySelector(`[role="radiogroup"][aria-label="${tool} permission"]`);
  const found = [...(group?.querySelectorAll<HTMLButtonElement>('[role="radio"]') ?? [])].find((each) => each.textContent.trim() === state);
  if (found === undefined) throw new Error(`no radio ${tool} ${state}`);
  return found;
}

const checked = (tool: string, state: string) => radio(tool, state).getAttribute('aria-checked') === 'true';

it('MCP-018: explains the server in plain words, with the command and the token limit', async () => {
  const target = await open();
  const text = target.querySelector('.intro')?.textContent ?? '';
  expect(text).toContain('anima mcp');
  expect(text).toContain('this computer');
  expect(text).toContain('still limits everything');
});

it('MCP-019: one group for each tool set, with its counts in words, and one row for each tool', async () => {
  const target = await open();
  const labels = [...target.querySelectorAll('h3 button .label')].map((each) => each.textContent);
  expect(labels).toEqual(['Devices: 1 allow, 0 ask, 2 off', 'Checks: 1 allow, 0 ask, 1 off', 'VPN: 1 allow, 0 ask, 0 off']);
  expect(target.querySelectorAll('li')).toHaveLength(6);
  expect(target.textContent).toContain('anima_check_update');
  expect(target.textContent).toContain('Check update');
});

it('MCP-019: each row has a kind marker, a sign-in marker where needed, a description and a radio group named for the tool', async () => {
  const target = await open();
  const rows = [...target.querySelectorAll('li')];
  const deletion = rows.find((row) => row.textContent.includes('anima_device_delete'));
  expect(deletion?.textContent).toContain('Change');
  expect(deletion?.textContent).toContain('Needs sign-in');
  expect(rows.find((row) => row.textContent.includes('anima_device_get'))?.textContent).not.toContain('Needs sign-in');
  expect(deletion?.querySelector('.about')?.textContent).toBe('About anima_device_delete.');
  const more = deletion?.querySelector<HTMLButtonElement>('.more');
  expect(more?.getAttribute('aria-expanded')).toBe('false');
  await click(more as HTMLElement);
  expect(more?.getAttribute('aria-expanded')).toBe('true');
  expect(deletion?.querySelector('.about')?.hasAttribute('hidden')).toBe(false);
  expect(checked('Device delete', 'Off')).toBe(true);
});

it('MCP-021: choosing a state shows it at once, saves one tool, and says "Saved" in a polite line', async () => {
  const target = await open();
  await click(radio('Check update', 'Ask'));
  expect(checked('Check update', 'Ask')).toBe(true);
  expect(shell.writes).toEqual([{ type: 'choose', names: ['anima_check_update'], state: 'ask' }]);
  expect(target.querySelector('[role="status"].saved')?.textContent).toBe('Saved. Check update: ask.');
  expect(target.querySelector('[role="alert"]')?.textContent).toBe('');
});

it('MCP-021: a state can be chosen by the arrow keys on the radio group', async () => {
  await open();
  radio('Check update', 'Off').dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true }));
  await settled();
  expect(checked('Check update', 'Ask')).toBe(true);
  expect(shell.writes).toHaveLength(1);
});

it('MCP-021: a refused write puts the control back and says so in an alert', async () => {
  const target = await open();
  shell.writeProblem = 'unwritable';
  await click(radio('Check update', 'Allow'));
  expect(checked('Check update', 'Off')).toBe(true);
  expect(checked('Check update', 'Allow')).toBe(false);
  expect(target.querySelector('[role="alert"]')?.textContent).toContain('Not saved. The permissions could not be saved');
  expect(target.querySelector('[role="alert"]')?.textContent).toContain('went back');
});

it('MCP-020: "Ask for all" applies at once, saves once, and one line announces the result', async () => {
  const target = await open();
  await click(button('Ask for all, all tools'));
  expect(document.querySelector('[role="dialog"]')).toBeNull();
  expect(shell.writes).toEqual([{ type: 'bulk', state: 'ask' }]);
  expect(checked('Device delete', 'Ask')).toBe(true);
  expect(checked('Vpn status', 'Ask')).toBe(true);
  expect(target.querySelectorAll('[role="status"]')).toHaveLength(1);
  expect(target.querySelector('[role="status"]')?.textContent).toBe('Saved. Every tool set to ask (6 tools).');
});

it('MCP-020: "Turn off all changes" makes the panel read-only at once, leaving read tools as they are', async () => {
  await open();
  await click(button('Ask for all, all tools'));
  await click(button('Turn off all changes, all tools'));
  expect(shell.writes.at(-1)).toEqual({ type: 'bulk', kind: 'change', state: 'off' });
  expect(checked('Device create', 'Off')).toBe(true);
  expect(checked('Device get', 'Ask')).toBe(true);
});

it('MCP-020: "Reset" returns every tool to its default', async () => {
  await open();
  await click(button('Allow all, Devices tools'));
  await click(button('Allow all changes'));
  await click(button('Reset, all tools'));
  expect(shell.writes.at(-1)).toEqual({ type: 'reset' });
  expect(checked('Device create', 'Off')).toBe(true);
  expect(checked('Device get', 'Allow')).toBe(true);
});

it('MCP-020: "Allow all" asks first; Cancel saves nothing and changes nothing', async () => {
  await open();
  await click(button('Allow all, all tools'));
  const dialog = document.querySelector('[role="dialog"]');
  expect(dialog?.textContent).toContain('change this hub without asking');
  expect(shell.writes).toHaveLength(0);
  await click(button('Cancel'));
  expect(shell.writes).toHaveLength(0);
  expect(checked('Device create', 'Off')).toBe(true);
});

it('MCP-020: confirming "Allow all" saves it and turns every tool on', async () => {
  await open();
  await click(button('Allow all, all tools'));
  await click(button('Allow all changes'));
  expect(shell.writes).toEqual([{ type: 'bulk', state: 'allow' }]);
  expect(checked('Device delete', 'Allow')).toBe(true);
  expect(document.querySelector('[role="dialog"]')).toBeNull();
});

it('MCP-019: a set has its own bulk actions that reach only its tools, and no Reset', async () => {
  await open();
  expect(document.querySelectorAll('[aria-label="Bulk actions, Checks tools"] button')).toHaveLength(3);
  await click(button('Ask for all, Checks tools'));
  expect(shell.writes).toEqual([{ type: 'bulk', toolset: 'checks', state: 'ask' }]);
  expect(checked('Check list', 'Ask')).toBe(true);
  expect(checked('Device get', 'Allow')).toBe(true);
  expect(document.querySelector('h3 button .label')?.textContent).toContain('1 allow');
});

it('MCP-021: when the file cannot be read, the section says so without a path and offers a retry', async () => {
  shell.readProblem = 'unreadable';
  const target = await open();
  expect(target.querySelector('[role="alert"]')?.textContent).toContain('cannot be read');
  expect(target.textContent).not.toContain('config.json');
  expect(target.querySelectorAll('li')).toHaveLength(0);
  shell.readProblem = null;
  await click(button('Try again'));
  expect(target.querySelectorAll('li')).toHaveLength(6);
  expect(HUB).toBe(mocks.hub.address.origin);
});
