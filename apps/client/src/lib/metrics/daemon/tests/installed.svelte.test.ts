// @vitest-environment jsdom
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CODE, showPending, ENROLMENT, click, command, pass, setUp, show, tearDown, text } from './panel.fixture';

vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

beforeEach(setUp);
afterEach(tearDown);

describe('Install daemon: an installed daemon (MET-018, MET-013)', () => {
  const status = (metricsStale: boolean) => ({ DaemonStatus: { device: { id: 'd1', metricsSource: 'DAEMON', metricsStale } } });

  it('MET-018: shows the source as set by the daemon and stale as a plain status, not an alert', async () => {
    const { fake } = await show(status(true), { source: 'DAEMON' });
    expect(fake.requests).toEqual([{ operation: 'DaemonStatus', variables: { id: 'd1' } }]);
    expect(text()).toContain('Metrics come from the daemon on');
    expect(document.querySelector('[role="status"]')?.textContent).toContain('Metrics are stale');
    expect(document.querySelector('[role="alert"]')).toBeNull();
  });

  it('MET-018: the "Reading daemon state." line is plain text, not a live region', async () => {
    const answer = await showPending('daemonStatus', 'DAEMON');
    const line = [...document.querySelectorAll('p')].find((each) => each.textContent.includes('Reading daemon state'));
    expect(line?.getAttribute('role')).toBeNull();
    await answer({ ok: true, value: { metricsSource: 'DAEMON', metricsStale: false, daemonVersion: null } });
  });

  it('MET-018: a current daemon reads as current', async () => {
    await show(status(false), { source: 'DAEMON' });
    expect(text()).toContain('Metrics are current.');
  });

  it('R-122: shows the version the daemon reported', async () => {
    await show({ DaemonStatus: { device: { id: 'd1', metricsSource: 'DAEMON', metricsStale: false, daemonVersion: '0.3.1' } } }, { source: 'DAEMON' });
    expect(text()).toContain('Daemon version 0.3.1.');
  });

  it('R-122: says so when a daemon older than R-122 reported no version', async () => {
    await show(status(false), { source: 'DAEMON' });
    expect(text()).toContain('The daemon has not reported its version.');
  });

  it('MET-018: when the state cannot be read the panel says so and still offers Revoke', async () => {
    await show({ DaemonStatus: { code: 'INTERNAL', message: 'down' } }, { source: 'DAEMON' });
    expect(text()).toContain('Could not read daemon state.');
    expect(text()).toContain('Revoke daemon');
  });

  it('MET-013, D-7: confirming revokes the token and the panel then shows stale', async () => {
    const { fake } = await show(
      { ...status(false), RevokeDaemonToken: { revokeDaemonToken: { id: 'd1', metricsSource: 'DAEMON', metricsStale: true } } },
      { source: 'DAEMON' },
    );
    await click('Revoke daemon');
    await click('Revoke daemon', '[role="dialog"]');
    expect(fake.requests.at(-1)).toEqual({ operation: 'RevokeDaemonToken', variables: { deviceId: 'd1' } });
    expect(text()).toContain('Daemon revoked. Metrics stay stale until it enrols again or you change the source.');
  });
});

describe('Install daemon: revoke and install again (MET-013, MET-019, D-7)', () => {
  const status = (metricsStale: boolean) => ({ DaemonStatus: { device: { id: 'd1', metricsSource: 'DAEMON', metricsStale } } });

  it('MET-013, R-105: the revoke dialog says the VPN access stays and the Headscale node must be deleted too', async () => {
    await show(status(false), { source: 'DAEMON' });
    await click('Revoke daemon');
    const dialog = document.querySelector('[role="dialog"]')?.textContent ?? '';
    expect(dialog).toContain('VPN access stays');
    expect(dialog).toContain('also delete its node in Headscale');
  });

  it('MET-013: a refused revoke keeps the dialog open and shows the hub message', async () => {
    await show({ ...status(false), RevokeDaemonToken: { code: 'FORBIDDEN', message: 'Not allowed.' } }, { source: 'DAEMON' });
    await click('Revoke daemon');
    await click('Revoke daemon', '[role="dialog"]');
    expect(document.querySelector('[role="dialog"] [role="alert"]')?.textContent).toBe('Not allowed.');
    expect(text()).not.toContain('Daemon revoked.');
  });

  it('MET-019, D-7: an DAEMON device can get a new install code, and Close returns to its state', async () => {
    const { fake } = await show({ ...status(true), ...ENROLMENT }, { source: 'DAEMON' });
    await click('Reinstall daemon');
    expect(fake.requests.at(-1)).toEqual({ operation: 'EnrolDaemon', variables: { deviceId: 'd1' } });
    expect(command()).toContain(CODE);
    expect(text()).not.toContain('Revoke daemon');
    await click('Close');
    expect(command()).toBe('');
    expect(text()).toContain('Revoke daemon');
    expect(text()).toContain('Metrics are stale');
  });
});

describe('Install daemon: revoked and used codes (MET-013, MET-012)', () => {
  const status = (metricsStale: boolean) => ({ DaemonStatus: { device: { id: 'd1', metricsSource: 'DAEMON', metricsStale } } });
  const REVOKED = { RevokeDaemonToken: { revokeDaemonToken: { id: 'd1', metricsSource: 'DAEMON', metricsStale: true } } };

  it('MET-013: after a new code the revoked sentence is gone, and Close reads the state again', async () => {
    const { fake } = await show({ ...status(true), ...REVOKED, ...ENROLMENT }, { source: 'DAEMON' });
    await click('Revoke daemon');
    await click('Revoke daemon', '[role="dialog"]');
    expect(document.body.textContent).toContain('Daemon revoked.');
    await click('Reinstall daemon');
    await click('Close');
    expect(document.body.textContent).not.toContain('Daemon revoked.');
    expect(fake.requests.filter((request) => request.operation === 'DaemonStatus')).toHaveLength(2);
  });

  it('MET-012: when the code expires the state is read again', async () => {
    const { fake } = await show({ ...status(false), ...ENROLMENT }, { source: 'DAEMON' });
    await click('Reinstall daemon');
    await pass(15 * 60);
    expect(fake.requests.filter((request) => request.operation === 'DaemonStatus')).toHaveLength(2);
  });

  it('MET-011: a device that was not a daemon device shows the daemon state once the hub says it enrolled', async () => {
    await show({ ...ENROLMENT, ...status(false) });
    await click('Install daemon');
    await click('Close');
    expect(document.body.textContent).toContain('Metrics are current.');
  });
});
