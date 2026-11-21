import { describe, expect, it } from 'vitest';
import { ago, folderName, groups, narrowed, parts, resultWords, rowsFrom, threadFrom, type ThreadRow, type ToolCall } from './thread';

const NOW = new Date(2026, 10, 20, 15, 0, 0);
const before = (minutes: number): string => new Date(NOW.getTime() - minutes * 60_000).toISOString();

function answer(id: string, last: string | null, more: Record<string, unknown> = {}): Record<string, unknown> {
  return { id, program: 'claude_code', title: `Thread ${id}`, folder: '/w/anima.ing', branch: 'dev', model: null, started: null, last, ...more };
}

function rows(...answers: Record<string, unknown>[]): readonly ThreadRow[] {
  const made = rowsFrom(answers);
  if (made === null) throw new Error('not rows');
  return made;
}

const call = (more: Partial<ToolCall>): ToolCall => ({ name: 'Bash', kind: 'command', summary: 'ls', input: '', patch: null, output: null, failed: false, ...more });

describe('the threads of this computer', () => {
  it('THR-011: an answer of the shell that is not a list of rows is refused whole', () => {
    expect(rowsFrom({})).toBeNull();
    expect(rowsFrom([answer('a', null), { id: 'b', program: 'other' }])).toBeNull();
    expect(rowsFrom([answer('a', 'not a time')])?.[0]?.last).toBeNull();
  });

  it('THR-013: the list has the groups changed now, today and earlier, and leaves an empty group out', () => {
    const list = rows(answer('now', before(1)), answer('today', before(90)), answer('old', before(3 * 24 * 60)), answer('never', null));
    expect(groups(list, NOW).map((group) => [group.name, group.rows.map((row) => row.id)])).toEqual([
      ['Changed now', ['now']],
      ['Today', ['today']],
      ['Earlier', ['old', 'never']],
    ]);
    expect(groups(rows(answer('old', before(3 * 24 * 60))), NOW).map((group) => group.name)).toEqual(['Earlier']);
  });

  it('THR-013: the search reads the title, the folder and the branch, and a program narrows the list', () => {
    const list = rows(answer('a', null, { title: 'Fix the sign-in test' }), answer('b', null, { program: 'codex', folder: '/w/site', branch: 'main' }));
    expect(narrowed(list, null, 'SIGN-IN').map((row) => row.id)).toEqual(['a']);
    expect(narrowed(list, null, 'site').map((row) => row.id)).toEqual(['b']);
    expect(narrowed(list, 'codex', '').map((row) => row.id)).toEqual(['b']);
    expect(narrowed(list, 'codex', 'sign-in')).toEqual([]);
  });

  it('THR-013: the time of a row is short words', () => {
    expect(ago(new Date(NOW.getTime() - 20_000), NOW)).toBe('now');
    expect(ago(new Date(NOW.getTime() - 42 * 60_000), NOW)).toBe('42 min');
    expect(ago(new Date(NOW.getTime() - 5 * 3_600_000), NOW)).toBe('5 h');
    expect(ago(null, NOW)).toBe('');
    expect(folderName('/w/anima.ing/')).toBe('anima.ing');
    expect(folderName(null)).toBe('No folder');
  });

  it('THR-014: tool calls that follow each other are one block, and an entry of an unknown kind is left out', () => {
    const thread = threadFrom({
      ...answer('a', null),
      prompts: 1,
      dropped: 0,
      entries: [
        { kind: 'prompt', at: null, text: 'Run the tests', images: 0 },
        { kind: 'tool', at: null, tool: call({}) },
        { kind: 'tool', at: null, tool: call({ kind: 'edit', name: 'Edit' }) },
        { kind: 'hologram', at: null },
        { kind: 'reply', at: null, text: 'Done.' },
        { kind: 'turn_end', at: null, stopped: true },
      ],
    });
    expect(thread).not.toBeNull();
    const shape = parts(thread?.entries ?? []).map((part) => (part.kind === 'tools' ? part.tools.length : part.entry.kind));
    expect(shape).toEqual(['prompt', 2, 'reply', 'turn_end']);
  });

  it('THR-014: the result of a call is the lines of its patch, a failure or the lines of its output', () => {
    expect(resultWords(call({ patch: '-a\n-b\n+c' }))).toBe('+1 −2');
    expect(resultWords(call({ failed: true, output: 'x' }))).toBe('Failed');
    expect(resultWords(call({ output: 'one\ntwo' }))).toBe('2 lines');
    expect(resultWords(call({}))).toBe('');
  });
});
