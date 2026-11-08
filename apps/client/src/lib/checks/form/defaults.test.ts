// @vitest-environment jsdom
import { expect, it, vi } from 'vitest';
import { failure, fakeHub, ok, settled, STORED } from '../defaults/defaults.fixture';
import { saved } from './check.fixture';
import { input, installFormHarness, show, type } from './form.fixture';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown }));

const NO_HUB = Symbol('no hub');

vi.mock('$lib/hub/context', () => ({
  useHub: () => {
    if (mocks.hub === NO_HUB) throw new Error('missing context');
    return mocks.hub;
  },
}));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../motion/engine.fixture')).fakeRun,
}));

installFormHarness();

const LABELS = ['Interval (seconds)', 'Retries before Down', 'Retry interval (seconds)', 'Degraded threshold (ms)'];
const K1 = ['60', '3', '20', '1000'];
const OWN = saved('TCP', { __typename: 'TcpConfig', port: 22, timeoutMs: 5000 }, { intervalSeconds: 90, retries: 1, retryIntervalSeconds: 15, degradedThresholdMs: 700 });

function settings(): string[] {
  return LABELS.map((label) => (input(label) as HTMLInputElement).value);
}

function open(answer: Parameters<typeof fakeHub>[0]['CheckDefaults']): void {
  mocks.hub = fakeHub({ CheckDefaults: answer }).hub;
  show();
}

it('D-26: a new check starts from the stored defaults', async () => {
  open(ok({ checkDefaults: STORED }));
  await settled();
  expect(settings()).toEqual(['120', '5', '45', '2500']);
});

it('D-26: an edited check keeps its own values and does not ask for the defaults', async () => {
  const hub = fakeHub({ CheckDefaults: ok({ checkDefaults: STORED }) });
  mocks.hub = hub.hub;
  show({ check: OWN });
  await settled();
  expect(settings()).toEqual(['90', '1', '15', '700']);
  expect(hub.sent).toEqual([]);
});

it('D-26: a failed defaults query leaves the K1 input defaults', async () => {
  open(failure('unreachable'));
  await settled();
  expect(settings()).toEqual(K1);
});

it('D-26: a form without a hub starts from the K1 input defaults', async () => {
  mocks.hub = NO_HUB;
  show();
  await settled();
  expect(settings()).toEqual(K1);
});

it('D-26: the form is there with the K1 values before a slow answer, and keeps what the operator typed', async () => {
  let release: (result: ReturnType<typeof ok>) => void = () => undefined;
  const answer = new Promise<ReturnType<typeof ok>>((resolve) => {
    release = resolve;
  });
  open(answer);
  expect(settings()).toEqual(K1);
  type('Retries before Down', '7');
  release(ok({ checkDefaults: STORED }));
  await settled();
  expect(settings()).toEqual(['120', '7', '45', '2500']);
});
