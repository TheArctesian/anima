import { describe, expect, it } from 'vitest';
import { askFrame, madeShell, saidFrame, stateFrame, THREAD } from './frames.fixture';
import { LiveThreads, REPLAY_MS } from './live.svelte';
import { OVER_LIMIT, type Outcome } from './run';

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
});
