// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { fakeUpdater, offering, recordingLog, scheduleHarness as setup } from './updates.fixture';
import { Updates } from '../updates.svelte';
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

const shell = vi.hoisted(() => ({ desktop: true, checks: 0 }));
vi.mock('$lib/desktop/token', () => ({ keyStoreAvailable: () => shell.desktop }));
vi.mock('$lib/desktop/updater', () => ({
  desktopUpdater: {
    check: () => {
      shell.checks += 1;
      return Promise.resolve(null);
    },
    restart: () => Promise.resolve(),
    currentVersion: () => Promise.resolve('1.0.0'),
  },
}));
vi.mock('$lib/desktop/process-log', () => ({ warnToProcessLog: () => undefined }));

describe('provideDesktopUpdates', () => {
  let app: ReturnType<typeof mount> | undefined;
  afterEach(async () => {
    if (app) await unmount(app);
    app = undefined;
    document.body.innerHTML = '';
    vi.unstubAllEnvs();
    shell.checks = 0;
  });

  function run(desktop: boolean, dev: boolean): void {
    shell.desktop = desktop;
    vi.stubEnv('DEV', dev);
    app = mount(DesktopProbe, { target: document.body });
    flushSync();
  }

  it('UPD-001: the desktop production app checks at start', () => {
    run(true, false);
    expect(shell.checks).toBe(1);
  });

  it('UPD-001: a dev shell does not check', () => {
    run(true, true);
    expect(shell.checks).toBe(0);
  });

  it('UPD-001: the web and iOS clients do not check', () => {
    run(false, false);
    expect(shell.checks).toBe(0);
  });
});
