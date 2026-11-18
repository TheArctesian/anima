import { expect, it, vi } from 'vitest';
import type { DeviceLinkStatus, LinkResult } from '$lib/desktop/link/shell';
import { HUB_SILENT, reachesHub, stepStates, waitForHub } from './steps';

const up = (path?: DeviceLinkStatus['path']): DeviceLinkStatus => ({ joined: true, state: 'up', ...(path === undefined ? {} : { path }) });

it('MOB-033: the steps before the current one are done, the ones after it wait', () => {
  expect(stepStates('hub', false)).toEqual({ invite: 'done', link: 'done', hub: 'now', 'sign-in': 'wait' });
  expect(stepStates('link', true)).toEqual({ invite: 'done', link: 'failed', hub: 'wait', 'sign-in': 'wait' });
});

it('LNK-006: the link reaches the hub when it is up with a path, or up with no path given', () => {
  expect([reachesHub(up('direct')), reachesHub(up('relayed')), reachesHub(up()), reachesHub(up('none'))]).toEqual([true, true, true, false]);
  expect(reachesHub({ joined: true, state: 'starting' })).toBe(false);
});

it('MOB-033: the wait ends when the link reaches the hub', async () => {
  const answers: LinkResult<DeviceLinkStatus>[] = [{ ok: false, problem: 'failed', message: '' }, { ok: true, value: up('none') }, { ok: true, value: up('direct') }];
  const read = vi.fn(() => Promise.resolve<LinkResult<DeviceLinkStatus>>(answers.shift() ?? { ok: true, value: up('direct') }));
  const pause = vi.fn(() => Promise.resolve());
  expect(await waitForHub({ read, pause, stopped: () => false })).toBeNull();
  expect(read).toHaveBeenCalledTimes(3);
});

it('MOB-033: a hub that never answers within a minute says why; Cancel ends the wait with no message', async () => {
  const read = () => Promise.resolve<LinkResult<DeviceLinkStatus>>({ ok: true, value: { joined: true, state: 'starting' } });
  const pause = vi.fn(() => Promise.resolve());
  expect(await waitForHub({ read, pause, stopped: () => false })).toBe(HUB_SILENT);
  expect(pause).toHaveBeenCalledTimes(60);
  expect(await waitForHub({ read, pause, stopped: () => true })).toBeNull();
});
