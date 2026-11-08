// @vitest-environment jsdom
import { expect, it, vi } from 'vitest';
import { failure, fakeHub, ok, settled, STORED } from './defaults.fixture';
import { defaultsReader } from './reader';
import { saved } from '../form/check.fixture';
import { choose, input, installFormHarness, show, type } from '../form/form.fixture';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown }));

const NO_HUB = Symbol('no hub');
const BROKEN = Symbol('broken');

vi.mock('$lib/hub/context', () => ({
  useHub: () => {
    if (mocks.hub === NO_HUB) throw new Error('missing_context');
    if (mocks.hub === BROKEN) throw new Error('broken context');
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

// The failed answer also carries a payload a wrong success check would read: a mutant that ignores `ok` shows STORED.
const POISONED = { ...failure('unreachable'), value: { checkDefaults: STORED } } as ReturnType<typeof failure>;

it('D-26: a failed defaults query leaves the K1 input defaults and the form editable', async () => {
  open(POISONED);
  await settled();
  expect(settings()).toEqual(K1);
  type('Retries before Down', '4');
  expect(settings()).toEqual(['60', '4', '20', '1000']);
});

it('D-26: a defaults query that throws leaves the K1 input defaults and the form editable', async () => {
  mocks.hub = { request: () => Promise.reject(new Error('socket closed')) };
  show();
  await settled();
  expect(settings()).toEqual(K1);
  type('Retries before Down', '4');
  expect(settings()).toEqual(['60', '4', '20', '1000']);
});

it('D-26: an error that is not a missing hub is not hidden', () => {
  mocks.hub = BROKEN;
  expect(() => defaultsReader()).toThrow('broken context');
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

it('D-26: a value typed back to what the form started with is the operator\'s own and survives a late answer', async () => {
  let release: (result: ReturnType<typeof ok>) => void = () => undefined;
  const answer = new Promise<ReturnType<typeof ok>>((resolve) => {
    release = resolve;
  });
  open(answer);
  type('Interval (seconds)', '75');
  type('Interval (seconds)', '60');
  release(ok({ checkDefaults: STORED }));
  await settled();
  expect(settings()).toEqual(['60', '5', '45', '2500']);
});

const interval = () => (input('Interval (seconds)') as HTMLInputElement).value;

it('D-29: an existing domain expiry check at 7200 keeps 7200', async () => {
  mocks.hub = fakeHub({ CheckDefaults: ok({ checkDefaults: STORED }) }).hub;
  const check = saved('DOMAIN_EXPIRY', { __typename: 'DomainExpiryConfig', domain: 'example.net', warnDays: 30, timeoutMs: 10000 }, { intervalSeconds: 7200 });
  show({ check });
  await settled();
  expect(interval()).toBe('7200');
});

it('D-29: late defaults while DOMAIN_EXPIRY is selected keep 12 hours, and another type then takes the hub default', async () => {
  open(ok({ checkDefaults: STORED }));
  choose('Type', 'DOMAIN_EXPIRY');
  await settled();
  expect(interval()).toBe('43200');
  choose('Type', 'TCP');
  expect(interval()).toBe('120');
});

it('D-29: a touched interval survives late defaults and a type round trip', async () => {
  open(ok({ checkDefaults: STORED }));
  type('Interval (seconds)', '7200');
  choose('Type', 'DOMAIN_EXPIRY');
  await settled();
  choose('Type', 'TCP');
  choose('Type', 'DOMAIN_EXPIRY');
  expect(interval()).toBe('7200');
});
