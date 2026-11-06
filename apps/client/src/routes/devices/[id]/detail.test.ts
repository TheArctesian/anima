import { describe, expect, it } from 'vitest';
import { fakeHub } from '$lib/devices/hub.fixture';
import type { HubResult } from '$lib/hub/hub';
import { Detail } from './detail.svelte';
import { parseDeviceId } from './device-id';
import { ID, OTHER_ID, answers, beatsPage, check, heartbeat } from './page.fixture.svelte';
import { Pages, type Connection } from './pages.svelte';

async function settle(): Promise<void> {
  for (let turn = 0; turn < 4; turn += 1) await Promise.resolve();
}

async function started(extra: Record<string, unknown> = {}) {
  const fake = fakeHub(answers(extra));
  const detail = new Detail(fake.hub, parseDeviceId(ID));
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
    const detail = new Detail(fake.hub, parseDeviceId(ID));
    detail.start();
    detail.stop();
    await settle();
    expect(detail.device).toBeNull();
    expect(fake.listeners('DeviceStatusChanged')).toBe(0);
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

  it('D-10: a gone owner is an empty list', async () => {
    const pages = new Pages<Row>(() => Promise.resolve({ ok: true, value: null }));
    await pages.load();
    expect(pages.items).toEqual([]);
  });
});
