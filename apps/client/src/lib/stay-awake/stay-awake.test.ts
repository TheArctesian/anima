import { describe, expect, it } from 'vitest';
import type { StayAwakeStatus } from '$lib/desktop/stay-awake/shell';
import { describeStayAwake, hubIsThisComputer, sleepNotice } from './stay-awake';

const status = (change: Partial<StayAwakeStatus>): StayAwakeStatus => ({ level: 'off', batteryFloor: null, held: 'off', pause: null, lid: 'ready', ...change });

describe('describeStayAwake', () => {
  it('AWK-002, AWK-003: says what the computer does at each held level', () => {
    expect(describeStayAwake(status({}))).toBe('This computer sleeps as usual.');
    expect(describeStayAwake(status({ level: 'awake', held: 'awake' }))).toBe('This computer does not sleep while it is idle. The display can sleep.');
    expect(describeStayAwake(status({ level: 'lid-closed', held: 'lid-closed' }))).toContain('stays on with the lid closed');
  });

  it('AWK-006: a low battery says the floor and when the control starts again', () => {
    const low = status({ level: 'awake', batteryFloor: 20, pause: 'low-battery' });
    expect(describeStayAwake(low)).toBe('Off for now: the battery is below 20%. It starts again on mains power.');
  });

  it('AWK-008: a lower level than the choice says the cause', () => {
    const lid = { level: 'lid-closed', held: 'awake', pause: 'lid-not-ready' } as const;
    expect(describeStayAwake(status({ ...lid, lid: 'needs-set-up' }))).toContain('not set up');
    expect(describeStayAwake(status({ ...lid, lid: 'ready' }))).toContain('refused the lid-closed level');
    expect(describeStayAwake(status({ level: 'awake', pause: 'unavailable' }))).toContain('refused to keep the computer awake');
  });
});

describe('AWK-005: the hub on this computer', () => {
  it('is this computer only when the hub is the one that runs here', () => {
    expect(hubIsThisComputer('http://127.0.0.1:8443', 'http://127.0.0.1:8443')).toBe(true);
    expect(hubIsThisComputer('http://127.0.0.1:8443/', 'http://127.0.0.1:8443')).toBe(true);
  });

  it('FRN-008: a loopback address of a joined device is not this computer when no hub runs here', () => {
    expect(hubIsThisComputer('http://127.0.0.1:8443', null)).toBe(false);
  });

  it('is not this computer for another port, another host or an address that is no URL', () => {
    expect(hubIsThisComputer('http://127.0.0.1:9000', 'http://127.0.0.1:8443')).toBe(false);
    expect(hubIsThisComputer('https://hub.anima.ing', 'http://127.0.0.1:8443')).toBe(false);
    expect(hubIsThisComputer('not a url', 'http://127.0.0.1:8443')).toBe(false);
    expect(hubIsThisComputer('http://127.0.0.1:8443', 'not a url')).toBe(false);
  });

  it('tells the operator that checks stop only when this computer is the hub and nothing holds it awake', () => {
    expect(sleepNotice(true, status({}))).toBe('This computer is the hub. Its checks stop while the computer sleeps.');
    expect(sleepNotice(true, status({ level: 'awake', pause: 'low-battery' }))).not.toBeNull();
    expect(sleepNotice(true, status({ level: 'awake', held: 'awake' }))).toBeNull();
    expect(sleepNotice(false, status({}))).toBeNull();
  });
});
