import { beforeEach, describe, expect, it, vi } from 'vitest';
import { desktopMcpShell, McpShellError, permissionsFrom } from './shell';
import { FIXTURE_TOOLS, HUB } from '$lib/mcp/mcp.fixture';

const shell = vi.hoisted(() => ({ invoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: shell.invoke }));

beforeEach(() => {
  shell.invoke.mockReset();
});

const answer = { hub: HUB, summary: 'Tool sets on: devices.', tools: FIXTURE_TOOLS };

describe('desktopMcpShell', () => {
  it('MCP-018: reads and writes through the two shell commands, naming the hub origin and nothing else', async () => {
    shell.invoke.mockResolvedValue(answer);
    await desktopMcpShell.read(HUB);
    expect(shell.invoke).toHaveBeenLastCalledWith('mcp_permissions_read', { hubOrigin: HUB });
    const change = { type: 'reset' } as const;
    await desktopMcpShell.write(HUB, change);
    expect(shell.invoke).toHaveBeenLastCalledWith('mcp_permissions_write', { hubOrigin: HUB, change });
  });

  it('MCP-018: the shell codes become typed errors with fixed text and no path', async () => {
    shell.invoke.mockRejectedValue({ code: 'unreadable', message: '/Users/ann/.config/anima/config.json is bad' });
    const failure = await desktopMcpShell.read(HUB).catch((reason: unknown) => reason);
    expect(failure).toBeInstanceOf(McpShellError);
    expect(failure).toMatchObject({ problem: 'unreadable' });
    expect(String(failure)).not.toContain('/Users');
    shell.invoke.mockRejectedValue('command not found');
    await expect(desktopMcpShell.read(HUB)).rejects.toMatchObject({ problem: 'failed' });
  });

  it('MCP-018: an answer that is not permissions is a failure, not a blank panel', async () => {
    shell.invoke.mockResolvedValue({ hub: HUB, summary: '', tools: [{ tool: 'x' }] });
    await expect(desktopMcpShell.read(HUB)).rejects.toMatchObject({ problem: 'failed' });
    expect(permissionsFrom(null)).toBeNull();
    expect(permissionsFrom(answer)).toEqual(answer);
  });
});
