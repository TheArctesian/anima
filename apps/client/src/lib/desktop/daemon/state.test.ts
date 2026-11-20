import { describe, expect, it } from 'vitest';
import { daemonHereLine, stateAfter } from './state.svelte';

describe('the state of the daemon of this computer (HMD-053)', () => {
  it('a daemon that was installed now, or reported already, is installed', () => {
    expect(stateAfter({ kind: 'installed' })).toEqual({ kind: 'installed' });
    expect(stateAfter({ kind: 'present' })).toEqual({ kind: 'installed' });
  });

  it('each end with no daemon has a reason in words for a person', () => {
    expect(stateAfter({ kind: 'not-ready' })).toEqual({ kind: 'failed', reason: 'The application did not find this computer among the devices of the hub.' });
    expect(stateAfter({ kind: 'failed', step: 'enrol' })).toEqual({ kind: 'failed', reason: 'The hub made no enrolment code.' });
    expect(stateAfter({ kind: 'failed', step: 'copy' })).toEqual({ kind: 'failed', reason: 'The daemon could not be copied to its installation folder.' });
  });

  it('each state has one sentence, and a failure says how to try again', () => {
    expect(daemonHereLine({ kind: 'idle' })).toContain('Open the Anima.ing application on a computer');
    expect(daemonHereLine({ kind: 'installing' })).toContain('is installing the daemon on this computer');
    expect(daemonHereLine({ kind: 'installed' })).toContain('The daemon of this computer is installed.');
    expect(daemonHereLine({ kind: 'failed', reason: 'X.' })).toBe(
      'The daemon was not installed on this computer. X. Quit the application and open it again to try once more.',
    );
  });
});
