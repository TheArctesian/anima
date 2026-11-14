import { describe, expect, it } from 'vitest';
import { FIXTURE_TOOLS } from './mcp.fixture';
import { applyChange, bulkActions, controlOf, countChanged, countsText, countStates, describeChange, groupLabel, groupTools, nameChange, needsConfirmation, TOOLSET_COUNT, toolLabel, type McpChange, type McpTool } from './permissions';

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
    expect(bulkActions().map((action) => action.label)).toEqual(['Allow all', 'Ask for all', 'Read-only (turn off all changes)', 'Reset']);
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

describe('changes from the default', () => {
  it('MCP-019: counts the tools that are not in their default state, read tools by allow and change tools by off', () => {
    expect(countChanged(FIXTURE_TOOLS)).toBe(0);
    const next = applyChange(FIXTURE_TOOLS, { type: 'bulk', toolset: 'checks', state: 'ask' });
    expect(countChanged(next)).toBe(2);
    expect(countChanged(applyChange(next, { type: 'reset' }))).toBe(0);
  });

  it('MCP-019: a group header adds the count only when it is above zero', () => {
    const [devices] = groupTools(FIXTURE_TOOLS);
    expect(groupLabel(devices as never)).toBe('Devices: 1 allow, 0 ask, 2 off');
    const [changed] = groupTools(applyChange(FIXTURE_TOOLS, { type: 'choose', names: ['anima_device_create'], state: 'allow' }));
    expect(groupLabel(changed as never)).toBe('Devices: 2 allow, 0 ask, 1 off; 1 changed from default');
  });

  it('MCP-019: thirteen tool sets are known, for the placeholder list', () => {
    expect(TOOLSET_COUNT).toBe(13);
  });
});

describe('failures name their control', () => {
  it('MCP-021: the same tool or the same bulk button is the same control', () => {
    expect(controlOf({ type: 'choose', names: ['a'], state: 'ask' })).toBe(controlOf({ type: 'choose', names: ['a'], state: 'ask' }));
    expect(controlOf({ type: 'choose', names: ['a'], state: 'ask' })).not.toBe(controlOf({ type: 'choose', names: ['b'], state: 'ask' }));
    expect(controlOf({ type: 'bulk', toolset: 'web', state: 'ask' })).not.toBe(controlOf({ type: 'bulk', state: 'ask' }));
  });

  it('MCP-021: says what the change tried to do', () => {
    expect(nameChange({ type: 'choose', names: ['anima_check_update'], state: 'allow' })).toBe('Check update to allow');
    expect(nameChange({ type: 'bulk', toolset: 'checks', state: 'ask' })).toBe('Every Checks tool to ask');
    expect(nameChange({ type: 'bulk', kind: 'change', state: 'off' })).toBe('Every change tool to off');
    expect(nameChange({ type: 'reset' })).toBe('Reset to default');
  });
});
