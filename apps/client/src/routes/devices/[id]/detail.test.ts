import { describe, expect, it } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import type { HubResult } from '$lib/hub/hub';
import { Detail } from './detail.svelte';
import { parseDeviceId, type DeviceId } from './device-id';
import { ID, OTHER_ID, answers, beatsPage, check, device, heartbeat } from './page.fixture.svelte';
import { Pages, type Connection } from './pages.svelte';

/** The well-formed ID the fixtures use; a test that cannot parse it fails at once. */
function deviceId(text: string): DeviceId {
  const parsed = parseDeviceId(text);
  if (parsed === null) throw new Error(`not a device ID: ${text}`);
  return parsed;
}

const DEVICE = deviceId(ID);
const NOW = () => new Date('2026-11-05T09:05:00Z');

async function settle(): Promise<void> {
  for (let turn = 0; turn < 4; turn += 1) await Promise.resolve();
}

async function started(extra: Record<string, unknown> = {}) {
  const fake = fakeHub(answers(extra));
  const detail = new Detail(fake.hub, DEVICE, NOW);
  detail.start();
  await detail.loadChecks();
  await settle();
  return { fake, detail };
}

describe('Detail live updates (D-11)', () => {
  it('D-11: a heartbeat of another device, or an older one, leaves the row as it is', async () => {
    const { fake, detail } = await started();
    fake.push('HeartbeatAdded', { heartbeatAdded: heartbeat('2026-11-04T10:05:00Z', 99, { deviceId: OTHER_ID }) });
    fake.push('HeartbeatAdded', { heartbeatAdded: heartbeat('2026-11-04T09:00:00Z', 99) });
    expect(detail.checks?.[0]?.lastHeartbeat?.responseTimeMs).toBe(12);
    fake.push('HeartbeatAdded', { heartbeatAdded: heartbeat('2026-11-04T10:05:00Z', 99, { heartbeat: { at: '2026-11-04T10:05:00Z', status: 'DOWN', responseTimeMs: null } }) });
    expect(detail.checks?.[0]).toMatchObject({ status: 'DOWN', lastHeartbeat: { status: 'DOWN', responseTimeMs: null } });
  });

  it('D-11: a heartbeat joins the open history once, and not another check\'s history', async () => {
    const { fake, detail } = await started({ CheckHeartbeats: { check: { id: 'c1', heartbeats: beatsPage(['2026-11-04T10:00:00Z']) } } });
    detail.openHistory('c1');
    await settle();
    fake.push('HeartbeatAdded', { heartbeatAdded: heartbeat('2026-11-04T10:01:00Z', 30) });
    fake.push('HeartbeatAdded', { heartbeatAdded: heartbeat('2026-11-04T10:01:00+00:00', 30) });
    fake.push('HeartbeatAdded', { heartbeatAdded: heartbeat('2026-11-04T10:02:00Z', 30, { checkId: 'c9' }) });
    expect(detail.history?.pages.items?.map((beat) => beat.at)).toEqual(['2026-11-04T10:01:00Z', '2026-11-04T10:00:00Z']);
  });

  it('D-11: after stop no update and no late answer changes the page', async () => {
    const fake = fakeHub(answers());
    const detail = new Detail(fake.hub, DEVICE, NOW);
    detail.start();
    detail.stop();
    await settle();
    expect(detail.device).toBeNull();
    expect(fake.listeners('DeviceStatusChanged')).toBe(0);
  });

  it('D-11: checks that answer after stop are dropped', async () => {
    const fake = fakeHub(answers());
    const detail = new Detail(fake.hub, DEVICE, NOW);
    detail.start();
    const loading = detail.loadChecks();
    detail.stop();
    await loading;
    expect(detail.checks).toBeNull();
  });

  it('D-11: a live beat older than the top of the history is left out', async () => {
    const { fake, detail } = await started({ CheckHeartbeats: { check: { id: 'c1', heartbeats: beatsPage(['2026-11-04T10:00:00Z']) } } });
    detail.openHistory('c1');
    await settle();
    fake.push('HeartbeatAdded', { heartbeatAdded: heartbeat('2026-11-04T09:59:00Z', 30) });
    expect(detail.history?.pages.items?.map((beat) => beat.at)).toEqual(['2026-11-04T10:00:00Z']);
  });

  it('D-10: deleting the check whose history is open closes the history', async () => {
    const { detail } = await started({ CheckHeartbeats: { check: { id: 'c1', heartbeats: beatsPage([]) } } });
    detail.openHistory('c1');
    detail.dropCheck('c1');
    expect(detail.history).toBeNull();
    expect(detail.checks).toEqual([]);
  });

  it('6.4: a returned check replaces its old row or joins the list', async () => {
    const { detail } = await started();
    detail.putCheck(check({ name: 'renamed' }));
    detail.putCheck(check({ id: 'c2' }));
    expect(detail.checks?.map((known) => [known.id, known.name])).toEqual([
      ['c1', 'renamed'],
      ['c2', 'ping'],
    ]);
  });
});

describe('Pages (D-10)', () => {
  interface Row {
    readonly id: string;
  }
  const page = (ids: string[], hasNextPage: boolean): HubResult<Connection<Row>> => ({
    ok: true,
    value: { pageInfo: { hasNextPage, endCursor: ids.at(-1) ?? null }, edges: ids.map((id) => ({ node: { id } })) },
  });

  it('D-10: Load older appends the next page after the last cursor', async () => {
    const asked: (string | null)[] = [];
    const pages = new Pages<Row>((after) => {
      asked.push(after);
      return Promise.resolve(after === null ? page(['a', 'b'], true) : page(['c'], false));
    });
    await pages.load();
    await pages.loadOlder();
    await pages.loadOlder();
    expect(asked).toEqual([null, 'b']);
    expect(pages.items?.map((row) => row.id)).toEqual(['a', 'b', 'c']);
    expect(pages.hasOlder).toBe(false);
  });

  it('D-10: a page that answers after stop is dropped', async () => {
    let answer: (result: HubResult<Connection<Row>>) => void = () => undefined;
    const pages = new Pages<Row>(() => new Promise((resolve) => (answer = resolve)));
    const loading = pages.load();
    pages.stop();
    answer(page(['a'], false));
    await loading;
    expect(pages.items).toBeNull();
  });

  it('D-10: a second Load older while one is on its way asks nothing; a reload clears the flag', async () => {
    const asked: (string | null)[] = [];
    let answer: (result: HubResult<Connection<Row>>) => void = () => undefined;
    const pages = new Pages<Row>((after) => {
      asked.push(after);
      return after === null ? Promise.resolve(page(['a'], true)) : new Promise((resolve) => (answer = resolve));
    });
    await pages.load();
    void pages.loadOlder();
    void pages.loadOlder();
    expect(asked).toEqual([null, 'a']);
    await pages.load();
    expect(pages.loadingOlder).toBe(false);
    answer(page(['b'], false));
  });

  it('D-10: a gone owner is an empty list', async () => {
    const pages = new Pages<Row>(() => Promise.resolve({ ok: true, value: null }));
    await pages.load();
    expect(pages.items).toEqual([]);
  });
});

const requested = (fake: ReturnType<typeof fakeHub>, operation: string) => fake.requests.filter((request) => request.operation === operation);

describe('Detail tabs (D-9)', () => {
  it('D-9: a tab loads its data the first time it shows, and not again', async () => {
    const fake = fakeHub(answers());
    const detail = new Detail(fake.hub, DEVICE, NOW);
    detail.showTab('overview');
    detail.showTab('checks');
    detail.showTab('overview');
    await settle();
    expect(requested(fake, 'DeviceChecks')).toHaveLength(1);
    detail.showTab('settings');
    detail.showTab('settings');
    expect(requested(fake, 'DeviceChecks')).toHaveLength(1);
  });

  it('D-9: the Sessions tab asks for the first page of this device\'s sessions, once', async () => {
    const fake = fakeHub(answers({ DeviceSessions: { device: { id: ID, sessions: { pageInfo: { hasNextPage: false, endCursor: null }, edges: [] } } } }));
    const detail = new Detail(fake.hub, DEVICE, NOW);
    detail.showTab('sessions');
    detail.showTab('sessions');
    await settle();
    expect(requested(fake, 'DeviceSessions')).toEqual([{ operation: 'DeviceSessions', variables: { deviceId: ID, first: 50, after: null } }]);
  });
});

describe('Detail metrics (D-5)', () => {
  const withMetrics = () =>
    fakeHub(answers({ Device: { device: device({ metricsSource: 'SSH' }) }, DeviceMetrics: { device: { id: ID, metricsSource: 'SSH', gpus: [], metrics: [], storage: { drives: [] }, readings: null } } }));

  it('D-5: the metrics load when the range changes and not when it stays the same', async () => {
    const fake = withMetrics();
    const detail = new Detail(fake.hub, DEVICE, NOW);
    detail.start();
    await settle();
    detail.showMetrics('overview', 'HOUR');
    detail.showMetrics('overview', 'HOUR');
    detail.showMetrics('overview', 'DAY');
    await settle();
    expect(requested(fake, 'DeviceMetrics').map((request) => request.variables)).toEqual([
      { id: ID, range: 'HOUR' },
      { id: ID, range: 'DAY' },
    ]);
  });

  it('D-5: leaving the Overview stops the live points, and coming back asks again', async () => {
    const fake = withMetrics();
    const detail = new Detail(fake.hub, DEVICE, NOW);
    detail.start();
    await settle();
    detail.showMetrics('overview', 'HOUR');
    await settle();
    expect(fake.listeners('MetricsAdded')).toBe(1);
    detail.showMetrics('checks', 'HOUR');
    expect(fake.listeners('MetricsAdded')).toBe(0);
    detail.showMetrics('overview', 'HOUR');
    await settle();
    expect(requested(fake, 'DeviceMetrics')).toHaveLength(2);
  });

  it('D-5: a device without a metrics source sends no metrics request', async () => {
    const fake = fakeHub(answers());
    const detail = new Detail(fake.hub, DEVICE, NOW);
    detail.start();
    await settle();
    detail.showMetrics('overview', 'HOUR');
    expect(requested(fake, 'DeviceMetrics')).toEqual([]);
  });
});

describe('Detail.deleteDevice (DEV-014)', () => {
  it('DEV-014: onDeleted runs after the hub deleted the device', async () => {
    const fake = fakeHub(answers({ DeleteDevice: { deleteDevice: ID } }));
    const deleted: string[] = [];
    const result = await new Detail(fake.hub, DEVICE, NOW).deleteDevice(() => deleted.push(ID));
    expect(result.ok).toBe(true);
    expect(deleted).toEqual([ID]);
  });

  it('DEV-014: onDeleted does not run when the hub fails', async () => {
    const fake = fakeHub(answers());
    const deleted: string[] = [];
    const result = await new Detail(fake.hub, DEVICE, NOW).deleteDevice(() => deleted.push(ID));
    expect(result.ok).toBe(false);
    expect(deleted).toEqual([]);
  });
});
