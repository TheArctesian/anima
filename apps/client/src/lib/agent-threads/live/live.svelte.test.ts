import { describe, expect, it } from 'vitest';
import { askFrame, madeShell, saidFrame, stateFrame, THREAD } from './frames.fixture';
import { LiveThreads, REPLAY_GAP_MS, REPLAY_MS } from './live.svelte';
import { TEXT_CHARS, waitingWords } from './timeline';
import { OVER_LIMIT, outcomeFrom, type Outcome } from './run';

const AWAY = { front: false, chosen: null };

function clock(start = 0): { now: () => Date; pass: (ms: number) => void } {
  let at = start;
  return {
    now: () => new Date(at),
    pass: (ms) => {
      at += ms;
    },
  };
}

describe('the threads that the runner drives', () => {
  it('TH14: updates fold into the thread; a repeat of the watch changes nothing', async () => {
    const shell = madeShell();
    const live = new LiveThreads(shell, () => AWAY, clock().now);
    await live.begin();
    shell.emit(stateFrame(1, 'working'));
    shell.emit(saidFrame(2, { kind: 'message', text: 'Done.', parent: null }));
    shell.emit(saidFrame(2, { kind: 'message', text: 'Done.', parent: null }));
    const thread = live.threads.get(THREAD);
    expect(thread?.entries.map((entry) => entry.kind)).toEqual(['reply']);
    expect(thread?.row.state).toBe('working');
  });

  it('THR-059: a request notifies once, and not while the watch replays what the runner kept', async () => {
    const shell = madeShell();
    const time = clock();
    const live = new LiveThreads(shell, () => AWAY, time.now);
    await live.begin();
    shell.link(true);
    shell.emit(stateFrame(1, 'working'));
    shell.emit(askFrame(2, 'q-1'));
    expect(shell.notices).toEqual([]);
    time.pass(REPLAY_MS + 1);
    shell.emit(askFrame(3, 'q-2'));
    expect(shell.notices, 'a request already waited').toEqual([]);
    shell.emit(stateFrame(4, 'idle', { open: [] }));
    shell.emit(stateFrame(5, 'working'));
    shell.emit(askFrame(6, 'q-3'));
    expect(shell.notices).toEqual([
      ['Fix the test', 'Your turn'],
      ['Fix the test', 'Asks to run a command'],
    ]);
  });

  it('THR-059: a turn that ends says "Your turn", unless the person has the thread open in front', async () => {
    const shell = madeShell();
    let looking = { front: true, chosen: THREAD as string | null };
    const live = new LiveThreads(shell, () => looking, clock().now);
    await live.begin();
    shell.emit(stateFrame(1, 'working'));
    shell.emit(stateFrame(2, 'idle'));
    expect(shell.notices).toEqual([]);
    looking = { front: false, chosen: THREAD };
    shell.emit(stateFrame(3, 'working'));
    shell.emit(stateFrame(4, 'idle'));
    expect(shell.notices).toEqual([['Fix the test', 'Your turn']]);
  });

  it('THR-055: a thread that changes while the person looks at it is seen; one in the back is not', async () => {
    const shell = madeShell();
    let looking = { front: true, chosen: THREAD as string | null };
    const live = new LiveThreads(shell, () => looking, clock().now);
    const seen: string[] = [];
    live.seenOpen = (id) => seen.push(id);
    await live.begin();
    shell.emit(stateFrame(1, 'idle'));
    expect(seen).toEqual([THREAD]);
    looking = { front: false, chosen: THREAD };
    shell.emit(stateFrame(2, 'working'));
    expect(seen).toEqual([THREAD]);
  });
});

describe('the calls of the page to the runner', () => {
  it('THR-114: a start over the soft limit waits; raising the limit sends it again', async () => {
    let raised = false;
    const shell = madeShell((action): Outcome => {
      if (action.call === 'raise_limit') {
        raised = true;
        return { outcome: 'limits', limits: { depth: 5, children: 10, running: 15 } };
      }
      if (action.call === 'start') return raised ? { outcome: 'started', thread: THREAD } : { outcome: 'refused', why: 'over_limit' };
      return { outcome: 'ok' };
    });
    const live = new LiveThreads(shell, () => AWAY, clock().now);
    const start = { call: 'start', program: 'codex', folder: '/w', mode: 'full_access', model: null, effort: null, prompt: 'Go' } as const;
    expect(await live.call(start)).toEqual({ ok: false, words: OVER_LIMIT });
    expect(live.overLimit).toEqual(start);
    expect(await live.raiseLimit()).toEqual({ ok: true, outcome: { outcome: 'started', thread: THREAD } });
    expect(shell.calls.map((call) => call.call)).toEqual(['start', 'raise_limit', 'start']);
    expect(live.overLimit).toBeNull();
  });

  it('TH14: a refusal of the runner comes back in words', async () => {
    const live = new LiveThreads(madeShell(() => ({ outcome: 'refused', why: 'request_gone' })), () => AWAY);
    expect(await live.call({ call: 'interrupt', thread: THREAD })).toEqual({ ok: false, words: 'That request is answered, closed, or its program ended.' });
  });

  it('TH97: a starting thread says what it waits for and for how long', () => {
    const started = new Date('2026-11-21T10:00:00Z');
    expect(waitingWords('claude_code', started, new Date('2026-11-21T10:00:42Z'))).toMatch(/^Waiting for the first answer of the program: 42 s\. Claude Code reads its sign-in first/);
    expect(waitingWords('codex', started, new Date('2026-11-21T10:03:05Z'))).toContain('3 min 5 s. Codex loads its plugins first');
  });

  it('TH98: the words of an older holder show from the list and go when a stop leaves the list without them', async () => {
    const words = 'The holder of the threads is older than the daemon.';
    let older = true;
    const shell = madeShell((action) => (action.call === 'list' ? (older ? { outcome: 'threads', threads: [], holder: words } : { outcome: 'threads', threads: [] }) : { outcome: 'ok' }));
    const live = new LiveThreads(shell, () => AWAY, clock().now);
    await live.begin();
    expect(live.holder).toBe(words);
    shell.emit(stateFrame(1, 'working'));
    older = false;
    shell.emit(stateFrame(2, 'stopped'));
    await Promise.resolve();
    await Promise.resolve();
    expect(live.holder).toBeNull();
  });

  it('TH97: a program that does not answer is refused with the words of the runner, which name its path', async () => {
    const said = 'The Codex at /opt/homebrew/bin/codex did not answer in 20 seconds.';
    const answered = ['/Applications/ChatGPT.app/codex'];
    const read = outcomeFrom({ outcome: 'refused', why: { refusal: 'program_does_not_answer', said, answered } });
    expect(read).toEqual({ outcome: 'refused', why: 'program_does_not_answer', said, answered });
    const live = new LiveThreads(madeShell(() => read ?? { outcome: 'ok' }), () => AWAY);
    expect(await live.call({ call: 'start', program: 'codex', folder: '/w', mode: 'ask', model: null, effort: null, prompt: 'hi' })).toEqual({ ok: false, words: said, answered });
  });
});

describe('the page after a reload', () => {
  it('THR-059: a replay that lasts longer than the first quiet time notifies nothing', async () => {
    const shell = madeShell();
    const time = clock();
    const live = new LiveThreads(shell, () => AWAY, time.now);
    await live.begin();
    shell.link(true);
    time.pass(REPLAY_MS - 200);
    shell.emit(stateFrame(1, 'working'));
    time.pass(300);
    shell.emit(stateFrame(2, 'idle'));
    time.pass(300);
    shell.emit(askFrame(3, 'q-1'));
    expect(shell.notices).toEqual([]);
    time.pass(REPLAY_GAP_MS + 1);
    shell.emit(stateFrame(4, 'working', { open: [] }));
    shell.emit(askFrame(5, 'q-2'));
    expect(shell.notices).toEqual([['Fix the test', 'Asks to run a command']]);
  });

  it('THR-114: when the raise fails, the start still waits so the dialog can say why', async () => {
    const shell = madeShell((action): Outcome => (action.call === 'raise_limit' ? { outcome: 'refused', why: 'holder' } : { outcome: 'refused', why: 'over_limit' }));
    const live = new LiveThreads(shell, () => AWAY, clock().now);
    await live.call({ call: 'start', program: 'codex', folder: '/w', mode: 'full_access', model: null, effort: null, prompt: 'Go' });
    expect((await live.raiseLimit()).ok).toBe(false);
    expect(live.overLimit).not.toBeNull();
  });

  it('TH14: a long text of the program is held to a bound', async () => {
    const shell = madeShell();
    const live = new LiveThreads(shell, () => AWAY, clock().now);
    await live.begin();
    shell.emit(stateFrame(1, 'working'));
    shell.emit(saidFrame(2, { kind: 'message', text: 'x'.repeat(TEXT_CHARS + 10), parent: null }));
    const entry = live.threads.get(THREAD)?.entries[0];
    expect(entry?.kind === 'reply' ? entry.text.length : 0).toBe(TEXT_CHARS + 1);
  });
});
