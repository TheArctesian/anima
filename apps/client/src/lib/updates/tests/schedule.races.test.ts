// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { fakeUpdater, offering, recordingLog, scheduleHarness as setup } from './updates.fixture';
import { CHECK_FAILED_LINE, CHECK_INTERVAL_MS } from '../schedule';
import { Updates } from '../updates.svelte';
import { BROWSER, DESKTOP_MAC } from '$lib/platform/answers.fixture';
import type { Capabilities } from '$lib/platform/platform';
import DesktopProbe from './DesktopProbe.svelte';

describe('overlapping work', () => {
  it('UPD-001: a tick during downloading starts no second check', async () => {
    const s = setup();
    let finish: (() => void) | undefined;
    s.port.next = () => Promise.resolve(offering(s.port, '1.1.0', () => new Promise((resolve) => { finish = resolve; })));
    s.schedule.start();
    await vi.waitFor(() => { expect(s.last().state.kind).toBe('downloading'); });
    s.fire();
    expect(s.port.checks).toBe(1);
    finish?.();
    await vi.waitFor(() => { expect(s.last().state.kind).toBe('installed'); });
    expect(s.port.installs).toEqual(['1.1.0']);
  });

  it('UPD-001: stop then start runs a fresh schedule with an immediate check', async () => {
    const s = setup();
    s.schedule.start();
    await vi.waitFor(() => { expect(s.last().state.kind).toBe('idle'); });
    s.schedule.stop();
    s.schedule.start();
    await vi.waitFor(() => { expect(s.port.checks).toBe(2); });
    expect(s.timers.map((t) => t.cancelled)).toEqual([true, false]);
  });
});

describe('Updates store', () => {
  it('UPD-001: the function start returns stops the scheduled checks', () => {
    const cancelled: boolean[] = [];
    const updates = new Updates(fakeUpdater(), recordingLog(), () => () => { cancelled.push(true); });
    updates.start()();
    expect(cancelled).toEqual([true]);
  });

  it('UPD-006: a version that cannot be read leaves current null', async () => {
    const port = fakeUpdater();
    port.currentVersion = () => Promise.reject(new Error('x'));
    const updates = new Updates(port, recordingLog());
    await updates.loadVersion();
    expect(updates.current).toBeNull();
  });
});

const shell = vi.hoisted(() => ({ answers: undefined as Capabilities | undefined, checks: 0, loads: 0, broken: false, warn: vi.fn() }));
const updater = vi.hoisted(() => ({
  check: () => {
    shell.checks += 1;
    return Promise.resolve(null);
  },
  restart: () => Promise.resolve(),
  currentVersion: () => Promise.resolve('1.0.0'),
}));
vi.mock('$lib/platform/platform', () => ({ capabilities: () => shell.answers }));
vi.mock('$lib/desktop/updater', () => {
  // Runs when the updater module is first imported: the count says whether it was loaded at all (UPD-001).
  shell.loads += 1;
  return {
    // While `broken`, using the loaded module fails: the test simulates a failure when the updater is used, not a failed import.
    get desktopUpdater() {
      if (shell.broken) throw new Error('the updater did not load');
      return updater;
    },
  };
});
vi.mock('$lib/desktop/process-log', () => ({ warnToProcessLog: shell.warn }));

describe('provideDesktopUpdates', () => {
  let app: ReturnType<typeof mount> | undefined;
  afterEach(async () => {
    if (app) await unmount(app);
    app = undefined;
    document.body.innerHTML = '';
    vi.unstubAllEnvs();
    vi.useRealTimers();
    shell.checks = 0;
    shell.broken = false;
    shell.warn.mockClear();
  });

  function run(answers: Capabilities, dev: boolean): void {
    shell.answers = answers;
    vi.stubEnv('DEV', dev);
    app = mount(DesktopProbe, { target: document.body });
    flushSync();
  }

  it('UPD-001: the desktop production app checks at start, loading the updater to do it', async () => {
    run(DESKTOP_MAC, false);
    await vi.waitFor(() => {
      expect(shell.checks).toBe(1);
    });
    expect(shell.loads).toBe(1);
  });

  it('UPD-001, UPD-006: an updater that cannot be loaded fails the check, logs it, and the next tick checks again', async () => {
    vi.useFakeTimers({ toFake: ['setInterval', 'clearInterval'] });
    shell.broken = true;
    run(DESKTOP_MAC, false);
    await vi.waitFor(() => {
      expect(document.querySelector('[data-update-state]')?.textContent).toBe('failed');
    });
    expect(shell.warn).toHaveBeenCalledExactlyOnceWith(CHECK_FAILED_LINE);
    expect(shell.checks).toBe(0);
    shell.broken = false;
    vi.advanceTimersByTime(CHECK_INTERVAL_MS);
    await vi.waitFor(() => {
      expect(shell.checks).toBe(1);
    });
    await vi.waitFor(() => {
      expect(document.querySelector('[data-update-state]')?.textContent).toBe('idle');
    });
  });

  it('UPD-001: a dev shell does not check', async () => {
    run(DESKTOP_MAC, true);
    await settle();
    expect(shell.checks).toBe(0);
  });

  it('UPD-001: the web and iOS clients do not check', async () => {
    run(BROWSER, false);
    await settle();
    expect(shell.checks).toBe(0);
  });
});

/** Lets every pending import and promise run, so a check that was going to start has started. */
async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
}
