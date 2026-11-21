// @vitest-environment jsdom
import { flushSync, mount, tick, unmount } from 'svelte';
import { afterEach, describe, expect, it } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import type { RetentionPeriod } from '$lib/retention/api';
import { readKept } from './api';
import KeptCard from './KeptCard.svelte';
import { NOT_AVAILABLE, countText, historyText, type HubKept } from './kept';

let app: ReturnType<typeof mount> | undefined;

afterEach(async () => {
  if (app) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

const page = (checks: readonly number[], next: string | null, totalCount = 3) => ({
  devices: {
    totalCount,
    pageInfo: { hasNextPage: next !== null, endCursor: next },
    edges: checks.map((count) => ({ node: { checks: Array.from({ length: count }, (_, index) => ({ id: String(index) })) } })),
  },
});

const PERIODS: RetentionPeriod[] = [
  { subject: 'HEARTBEATS', days: 90, minDays: 1, maxDays: 365, defaultDays: 90 },
  { subject: 'METRICS', days: 30, minDays: 1, maxDays: 365, defaultDays: 30 },
  { subject: 'EVENTS', days: 30, minDays: 1, maxDays: 365, defaultDays: 30 },
];

describe('what the hub keeps (HMD-073)', () => {
  it('names the days of metrics and of check results, and says so when the hub gave neither', () => {
    expect(historyText(PERIODS)).toBe('30 days of metrics, 90 days of check results');
    expect(historyText([{ subject: 'EVENTS', days: 7 }])).toBe(NOT_AVAILABLE);
    expect(historyText(null)).toBe(NOT_AVAILABLE);
    expect(countText(1204)).toBe('1,204');
    expect(countText(null)).toBe(NOT_AVAILABLE);
  });

  it('counts the devices and the checks of each device and reads the retention periods', async () => {
    const { hub } = fakeHub({ HubKeptCounts: page([2, 0, 3], null), Retention: { retention: PERIODS } });
    expect(await readKept(hub)).toEqual({ devices: 3, checks: 5, periods: PERIODS });
  });

  it('gives no count of checks when a page fails, and no periods when the hub refuses them', async () => {
    const { hub } = fakeHub({ HubKeptCounts: { code: 'unreachable', message: 'No answer' }, Retention: { code: 'forbidden', message: 'No' } });
    expect(await readKept(hub)).toEqual({ devices: null, checks: null, periods: null });
  });

  it('shows the three figures, and the words of a figure that is not there', async () => {
    const kept: HubKept = { devices: 12, checks: null, periods: PERIODS };
    app = mount(KeptCard, { target: document.body, props: { read: () => Promise.resolve(kept) } });
    await tick();
    await tick();
    flushSync();
    const rows = [...document.querySelectorAll('dl > div')].map((row) => `${row.querySelector('dt')?.textContent ?? ''}: ${row.querySelector('dd')?.textContent ?? ''}`);
    expect(document.querySelector('h2')?.textContent).toBe('What the hub keeps');
    expect(rows).toEqual(['Devices: 12', `Checks: ${NOT_AVAILABLE}`, 'History: 30 days of metrics, 90 days of check results']);
  });

  it('says that each figure is not available when the reading fails', async () => {
    app = mount(KeptCard, { target: document.body, props: { read: () => Promise.reject(new Error('lost')) } });
    for (let turn = 0; turn < 3; turn += 1) await tick();
    flushSync();
    expect([...document.querySelectorAll('dd')].map((value) => value.textContent)).toEqual([NOT_AVAILABLE, NOT_AVAILABLE, NOT_AVAILABLE]);
  });
});
