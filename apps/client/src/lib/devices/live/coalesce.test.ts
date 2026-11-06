import { expect, it } from 'vitest';
import { coalesced } from './coalesce';

/** A run that finishes when the test says so. */
function slowRun() {
  const finishers: (() => void)[] = [];
  const run = () =>
    new Promise<void>((resolve) => {
      finishers.push(resolve);
    });
  return { run, finishers };
}

const turn = async () => {
  for (let round = 0; round < 4; round += 1) await Promise.resolve();
};

it('coalesced: calls during a run become one more run', async () => {
  const { run, finishers } = slowRun();
  const call = coalesced(run, () => false);
  void call();
  void call();
  void call();
  expect(finishers).toHaveLength(1);
  finishers[0]?.();
  await turn();
  expect(finishers).toHaveLength(2);
  finishers[1]?.();
  await turn();
  expect(finishers).toHaveLength(2);
});

it('coalesced: no extra run when the owner has stopped', async () => {
  const { run, finishers } = slowRun();
  const call = coalesced(run, () => true);
  void call();
  void call();
  finishers[0]?.();
  await turn();
  expect(finishers).toHaveLength(1);
});

it('coalesced: after a run ends, the next call runs at once', async () => {
  const { run, finishers } = slowRun();
  const call = coalesced(run, () => false);
  void call();
  finishers[0]?.();
  await turn();
  void call();
  expect(finishers).toHaveLength(2);
});
