import { describe, expect, it } from 'vitest';
import { fakeShell, HUB } from './mcp.fixture';
import { McpPermissionsStore } from './store.svelte';

const stateOf = (store: McpPermissionsStore, name: string) => store.tools.find((each) => each.tool === name)?.state;

async function loaded(shell = fakeShell()): Promise<{ store: McpPermissionsStore; shell: ReturnType<typeof fakeShell> }> {
  const store = new McpPermissionsStore(shell, HUB);
  await store.load();
  return { store, shell };
}

describe('McpPermissionsStore', () => {
  it('MCP-021: a change shows at once and is confirmed by the shell', async () => {
    const { store, shell } = await loaded();
    shell.hold = true;
    const saving = store.change({ type: 'choose', names: ['anima_check_update'], state: 'ask' });
    expect(stateOf(store, 'anima_check_update')).toBe('ask');
    // The write starts after the change is shown; wait until it is on its way.
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(stateOf(store, 'anima_check_update')).toBe('ask');
    shell.release();
    await saving;
    expect(stateOf(store, 'anima_check_update')).toBe('ask');
    expect(store.saved).toBe('Check update: ask.');
    expect(store.writeProblem).toBe('');
  });

  it('MCP-021: a refused change goes back and says so, and keeps the changes that were fine', async () => {
    const { store, shell } = await loaded();
    shell.writeProblem = 'unwritable';
    await store.change({ type: 'choose', names: ['anima_check_update'], state: 'allow' });
    expect(stateOf(store, 'anima_check_update')).toBe('off');
    expect(store.writeProblem).toContain('could not be saved');
    expect(store.saved).toBe('');
    shell.writeProblem = null;
    await store.change({ type: 'choose', names: ['anima_check_update'], state: 'ask' });
    expect(stateOf(store, 'anima_check_update')).toBe('ask');
    expect(store.writeProblem).toBe('');
  });

  it('MCP-021: changes reach the shell one at a time, in the order they were made', async () => {
    const { store, shell } = await loaded();
    const first = store.change({ type: 'bulk', state: 'ask' });
    const second = store.change({ type: 'choose', names: ['anima_vpn_status'], state: 'off' });
    expect(stateOf(store, 'anima_vpn_status')).toBe('off');
    await Promise.all([first, second]);
    expect(shell.writes.map((change) => change.type)).toEqual(['bulk', 'choose']);
    expect(stateOf(store, 'anima_device_get')).toBe('ask');
  });

  it('MCP-021: a file that cannot be read is a failed load with a path-free reason, and can be tried again', async () => {
    const shell = fakeShell();
    shell.readProblem = 'unreadable';
    const { store } = await loaded(shell);
    expect(store.status).toBe('failed');
    expect(store.loadProblem).toContain('cannot be read');
    shell.readProblem = null;
    await store.load();
    expect(store.status).toBe('ready');
    expect(store.tools).toHaveLength(6);
  });
});
