import { expect, it } from 'vitest';
import { failure, ok } from '$lib/access/access.fixture';
import type { Hub, HubResult } from '$lib/hub/hub';
import { ReleasePublishing } from '../publishing.svelte';

/** A hub that answers every request with `answer`. */
function answering(answer: HubResult<unknown>): Pick<Hub, 'request'> {
  return { request: () => Promise.resolve(answer) } as unknown as Pick<Hub, 'request'>;
}

it('PRO-037: the hub says whether it can publish', async () => {
  const on = new ReleasePublishing(answering(ok({ releasePublishing: true })));
  await on.load();
  expect(on.on).toBe(true);
  const off = new ReleasePublishing(answering(ok({ releasePublishing: false })));
  await off.load();
  expect(off.on).toBe(false);
});

it('PRO-038: a hub that cannot be asked counts as one that cannot publish', async () => {
  const publishing = new ReleasePublishing(answering(failure('unreachable')));
  await publishing.load();
  expect(publishing.on).toBe(false);
});
