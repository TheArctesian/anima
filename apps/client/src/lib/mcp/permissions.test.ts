import { describe, expect, it } from 'vitest';
import { FIXTURE_TOOLS } from './mcp.fixture';
import { applyChange, bulkActions, countsText, countStates, describeChange, groupTools, needsConfirmation, toolLabel, type McpChange, type McpTool } from './permissions';

const stateOf = (tools: readonly McpTool[], name: string) => tools.find((each) => each.tool === name)?.state;

describe('grouping and counts', () => {
  it('MCP-019: the tools are grouped by set in the command line order, each with its counts in words', () => {
    const groups = groupTools(FIXTURE_TOOLS);
    expect(groups.map((group) => group.label)).toEqual(['Devices', 'Checks', 'VPN']);
    expect(groups[0]?.counts).toEqual({ allow: 1, ask: 0, off: 2 });
    expect(countsText(countStates(FIXTURE_TOOLS))).toBe('3 allow, 0 ask, 3 off');
  });

  it('MCP-019: a set this client does not know is still shown, after the known ones', () => {
    const extra: McpTool = { ...(FIXTURE_TOOLS[0] as McpTool), toolset: 'printers', tool: 'anima_printer_list' };
    expect(groupTools([extra, ...FIXTURE_TOOLS]).map((group) => group.toolset)).toEqual(['devices', 'checks', 'vpn', 'printers']);
  });

  it('MCP-019: a tool id becomes plain words', () => {
    expect(toolLabel('anima_check_update')).toBe('Check update');
    expect(toolLabel('anima_vpn_approve_route')).toBe('Vpn approve route');
  });
});

describe('applyChange', () => {
  it('MCP-021: choosing one tool changes only that tool', () => {
    const next = applyChange(FIXTURE_TOOLS, { type: 'choose', names: ['anima_check_update'], state: 'ask' });
    expect(stateOf(next, 'anima_check_update')).toBe('ask');
    expect(next.filter((each) => each.state === 'ask')).toHaveLength(1);
  });

  it('MCP-020: a bulk change covers one set, one kind, or both', () => {
    const set = applyChange(FIXTURE_TOOLS, { type: 'bulk', toolset: 'devices', state: 'ask' });
    expect(set.filter((each) => each.state === 'ask').map((each) => each.toolset)).toEqual(['devices', 'devices', 'devices']);
    const changes = applyChange(FIXTURE_TOOLS, { type: 'bulk', kind: 'change', state: 'allow' });
    expect(stateOf(changes, 'anima_device_get')).toBe('allow');
    expect(changes.filter((each) => each.state === 'allow')).toHaveLength(6);
    const both = applyChange(FIXTURE_TOOLS, { type: 'bulk', toolset: 'checks', kind: 'change', state: 'ask' });
    expect(both.filter((each) => each.state === 'ask').map((each) => each.tool)).toEqual(['anima_check_update']);
  });

  it('MCP-020: reset gives read tools allow and change tools off', () => {
    const everything = applyChange(FIXTURE_TOOLS, { type: 'bulk', state: 'ask' });
    const next = applyChange(everything, { type: 'reset' });
    expect(next).toEqual(FIXTURE_TOOLS);
  });
});

describe('confirmation', () => {
  it('MCP-020: only allowing change tools in bulk asks first', () => {
    const [allow, ask, off, reset] = bulkActions().map((action) => action.change);
    expect(needsConfirmation(allow as McpChange)).toBe(true);
    expect(needsConfirmation(ask as McpChange)).toBe(false);
    expect(needsConfirmation(off as McpChange)).toBe(false);
    expect(needsConfirmation(reset as McpChange)).toBe(false);
    expect(needsConfirmation({ type: 'bulk', toolset: 'web', state: 'allow' })).toBe(true);
    expect(needsConfirmation({ type: 'bulk', kind: 'read', state: 'allow' })).toBe(false);
    expect(needsConfirmation({ type: 'choose', names: ['anima_check_update'], state: 'allow' })).toBe(false);
  });

  it('MCP-020: a set has the same bulk actions without Reset', () => {
    expect(bulkActions().map((action) => action.label)).toEqual(['Allow all', 'Ask for all', 'Turn off all changes', 'Reset']);
    expect(bulkActions('vpn').map((action) => action.label)).toEqual(['Allow all', 'Ask for all', 'Turn off changes']);
    expect(bulkActions('vpn')[2]?.change).toEqual({ type: 'bulk', toolset: 'vpn', kind: 'change', state: 'off' });
  });
});

describe('describeChange', () => {
  it('MCP-022: says what a change did, with the number of tools it covered', () => {
    expect(describeChange({ type: 'bulk', toolset: 'devices', state: 'ask' }, FIXTURE_TOOLS)).toBe('Devices: every tool set to ask (3 tools).');
    expect(describeChange({ type: 'bulk', kind: 'change', state: 'off' }, FIXTURE_TOOLS)).toBe('Every change tool set to off (3 tools).');
    expect(describeChange({ type: 'choose', names: ['anima_check_update'], state: 'allow' }, FIXTURE_TOOLS)).toBe('Check update: allow.');
  });
});
