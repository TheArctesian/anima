import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CHECK_FAILED_LINE, CHECK_INTERVAL_MS, INSTALL_FAILED_LINE } from '../schedule';
import { offering, scheduleHarness as setup, START } from './updates.fixture';

beforeEach(() => {
  vi.useRealTimers();
});
afterEach(() => {
  vi.useRealTimers();
});

describe('update checks', () => {
  it('UPD-001: checks at start and then every 4 hours', async () => {
    const s = setup();
    s.schedule.start();
    await vi.waitFor(() => { expect(s.last().state.kind).toBe('idle'); });
    expect(s.port.checks).toBe(1);
    expect(s.timers.map((t) => t.ms)).toEqual([4 * 60 * 60 * 1000]);
    await s.tick();
    await s.tick();
    expect(s.port.checks).toBe(3);
    expect(s.last().nextCheckAt).toBe(START + 3 * CHECK_INTERVAL_MS);
  });

  it('UPD-001: starting twice runs one schedule', async () => {
    const s = setup();
    s.schedule.start();
    s.schedule.start();
    await vi.waitFor(() => { expect(s.last().state.kind).toBe('idle'); });
    expect(s.timers).toHaveLength(1);
    expect(s.port.checks).toBe(1);
  });

});

describe('update install', () => {
  it('UPD-003: an offered update downloads and installs with no prompt, passing through downloading', async () => {
    const s = setup();
    s.port.next = () => Promise.resolve(offering(s.port, '1.1.0'));
    s.schedule.start();
    await vi.waitFor(() => { expect(s.last().state).toEqual({ kind: 'installed', version: '1.1.0' }); });
    expect(s.history.map((h) => h.state.kind)).toEqual(['checking', 'downloading', 'installed']);
    expect(s.port.installs).toEqual(['1.1.0']);
  });

  it('UPD-003: the schedule never restarts', async () => {
    const s = setup();
    s.port.next = () => Promise.resolve(offering(s.port, '1.1.0'));
    s.schedule.start();
    await vi.waitFor(() => { expect(s.last().state.kind).toBe('installed'); });
    expect(s.port.restarts).toBe(0);
  });

  it('UPD-005: once installed, the timer is cancelled and no check runs', async () => {
    const s = setup();
    s.port.next = () => Promise.resolve(offering(s.port, '1.1.0'));
    s.schedule.start();
    await vi.waitFor(() => { expect(s.last().state.kind).toBe('installed'); });
    expect(s.timers[0]?.cancelled).toBe(true);
    expect(s.last().nextCheckAt).toBeNull();
    await s.schedule.checkNow();
    expect(s.port.checks).toBe(1);
  });

});

describe('update failures and manual checks', () => {
  it('UPD-006: a failed check logs one fixed line, shows failed and the next scheduled check still runs', async () => {
    const s = setup();
    s.port.next = () => Promise.reject(new Error('https://user:pw@feed token=abc'));
    s.schedule.start();
    await vi.waitFor(() => { expect(s.last().state.kind).toBe('failed'); });
    expect(s.log.lines).toEqual([CHECK_FAILED_LINE]);
    expect(s.last().nextCheckAt).toBe(START + CHECK_INTERVAL_MS);
    s.port.next = () => Promise.resolve(null);
    await s.tick();
    expect(s.last().state.kind).toBe('idle');
    expect(s.port.checks).toBe(2);
  });

  it('UPD-002: an install that fails (a bad signature) logs one fixed line, installs nothing and keeps checking', async () => {
    const s = setup();
    s.port.next = () => Promise.resolve(offering(s.port, '1.1.0', () => Promise.reject(new Error('signature mismatch'))));
    s.schedule.start();
    await vi.waitFor(() => { expect(s.last().state.kind).toBe('failed'); });
    expect(s.log.lines).toEqual([INSTALL_FAILED_LINE]);
    expect(s.history.some((h) => h.state.kind === 'installed')).toBe(false);
    expect(s.timers[0]?.cancelled).toBe(false);
    await s.tick();
    expect(s.port.checks).toBe(2);
  });

});

describe('manual checks', () => {
  it('UPD-006: a manual check runs once now and does not move the 4-hour cadence', async () => {
    const s = setup();
    s.schedule.start();
    await vi.waitFor(() => { expect(s.last().state.kind).toBe('idle'); });
    const before = s.last().nextCheckAt;
    s.advance(60 * 60 * 1000);
    await s.schedule.checkNow();
    expect(s.port.checks).toBe(2);
    expect(s.last().nextCheckAt).toBe(before);
    expect(s.timers).toHaveLength(1);
  });

  it('UPD-006: a manual check does nothing while a check runs', async () => {
    const s = setup();
    let finish: (() => void) | undefined;
    s.port.next = () => new Promise((resolve) => { finish = () => { resolve(null); }; });
    const first = s.schedule.checkNow();
    await s.schedule.checkNow();
    expect(s.port.checks).toBe(1);
    finish?.();
    await first;
  });

  it('UPD-006: a manual check works before the schedule starts and after a failure', async () => {
    const s = setup();
    s.port.next = () => Promise.reject(new Error('down'));
    await s.schedule.checkNow();
    expect(s.last().state.kind).toBe('failed');
    s.port.next = () => Promise.resolve(null);
    await s.schedule.checkNow();
    expect(s.last().state.kind).toBe('idle');
  });

  it('UPD-001: stop cancels the timer', async () => {
    const s = setup();
    s.schedule.start();
    await vi.waitFor(() => { expect(s.last().state.kind).toBe('idle'); });
    s.schedule.stop();
    expect(s.timers[0]?.cancelled).toBe(true);
  });
});
