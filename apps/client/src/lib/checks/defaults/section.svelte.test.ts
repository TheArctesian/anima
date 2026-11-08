// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import { LIMITS } from '$lib/checks/form/model/range';
import DefaultsSection from './DefaultsSection.svelte';
import { failure, fakeHub, ok, settled, STORED } from './defaults.fixture';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown }));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;
let hub: ReturnType<typeof fakeHub>;

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

async function open(answers: Parameters<typeof fakeHub>[0]): Promise<HTMLElement> {
  hub = fakeHub(answers);
  mocks.hub = hub.hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(DefaultsSection, { target });
  flushSync();
  await settled();
  return target;
}

const LABELS = ['Interval (seconds)', 'Retries before Down', 'Retry interval (seconds)', 'Degraded threshold (ms)'];

function field(target: HTMLElement, label: string): HTMLInputElement {
  const found = [...target.querySelectorAll('label')].find((each) => each.textContent === label)?.control;
  if (found === null || found === undefined) throw new Error(`no field ${label}`);
  return found as HTMLInputElement;
}

function values(target: HTMLElement): string[] {
  return LABELS.map((label) => field(target, label).value);
}

function typeInto(input: HTMLInputElement, value: string): void {
  input.value = value;
  input.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
}

function save(target: HTMLElement): HTMLButtonElement {
  const found = target.querySelector<HTMLButtonElement>('button[type="submit"]');
  if (found === null) throw new Error('no save');
  return found;
}

function problems(target: HTMLElement): string[] {
  return [...target.querySelectorAll('.problem')].map((each) => each.textContent);
}

const SAVED = { intervalSeconds: 300, retries: 2, retryIntervalSeconds: 30, degradedThresholdMs: 4000 };

it('D-26: the section shows the four stored defaults, each with its bounds', async () => {
  const target = await open({ CheckDefaults: ok({ checkDefaults: STORED }) });
  expect(values(target)).toEqual(['120', '5', '45', '2500']);
  const { intervalSeconds, retries, retryIntervalSeconds, degradedThresholdMs } = LIMITS;
  expect([...target.querySelectorAll('.hint')].map((each) => each.textContent)).toEqual(
    [intervalSeconds, retries, retryIntervalSeconds, degradedThresholdMs].map(([min, max]) => `Allowed ${String(min)} to ${String(max)}.`),
  );
});

it('D-26: one Save sends all four values as numbers and then shows the values the hub returns', async () => {
  const target = await open({ CheckDefaults: ok({ checkDefaults: STORED }), SetCheckDefaults: ok({ setCheckDefaults: SAVED }) });
  typeInto(field(target, 'Interval (seconds)'), '240');
  typeInto(field(target, 'Retries before Down'), '2');
  save(target).click();
  await settled();
  expect(hub.sent.filter((each) => each.operation === 'SetCheckDefaults')).toEqual([
    { operation: 'SetCheckDefaults', variables: { input: { intervalSeconds: 240, retries: 2, retryIntervalSeconds: 45, degradedThresholdMs: 2500 } } },
  ]);
  expect(values(target)).toEqual(['300', '2', '30', '4000']);
});

it.each([
  ['Interval (seconds)', '9', 'Enter an interval in seconds from 10 to 86400.'],
  ['Interval (seconds)', '86401', 'Enter an interval in seconds from 10 to 86400.'],
  ['Retries before Down', '11', 'Enter the retries from 0 to 10.'],
  ['Retry interval (seconds)', '4', 'Enter a retry interval in seconds from 5 to 3600.'],
  ['Degraded threshold (ms)', '0', 'Enter a threshold in ms from 1 to 600000.'],
  ['Degraded threshold (ms)', 'abc', 'Enter a threshold in ms from 1 to 600000.'],
])('CHK-008: %s = %s is refused beside its input and nothing is sent', async (label, text, message) => {
  const target = await open({ CheckDefaults: ok({ checkDefaults: STORED }), SetCheckDefaults: ok({ setCheckDefaults: SAVED }) });
  const input = field(target, label);
  typeInto(input, text);
  save(target).click();
  await settled();
  expect(hub.sent.map((each) => each.operation)).toEqual(['CheckDefaults']);
  expect(input.getAttribute('aria-invalid')).toBe('true');
  const described = (input.getAttribute('aria-describedby') ?? '').split(' ').map((each) => document.getElementById(each)?.textContent);
  expect(described).toContain(message);
  expect(problems(target).filter((each) => each !== '')).toEqual([message]);
});

it('CHK-008: the lower and upper bounds themselves are sent', async () => {
  const target = await open({ CheckDefaults: ok({ checkDefaults: STORED }), SetCheckDefaults: ok({ setCheckDefaults: SAVED }) });
  typeInto(field(target, 'Interval (seconds)'), '10');
  typeInto(field(target, 'Retries before Down'), '0');
  typeInto(field(target, 'Retry interval (seconds)'), '3600');
  typeInto(field(target, 'Degraded threshold (ms)'), '600000');
  save(target).click();
  await settled();
  expect(hub.sent.at(-1)?.variables).toEqual({ input: { intervalSeconds: 10, retries: 0, retryIntervalSeconds: 3600, degradedThresholdMs: 600000 } });
});

it('D-26, LOG-007: a failed save shows a fixed message, never the hub text, and keeps the input', async () => {
  const target = await open({ CheckDefaults: ok({ checkDefaults: STORED }), SetCheckDefaults: failure('INTERNAL') });
  typeInto(field(target, 'Interval (seconds)'), '240');
  save(target).click();
  await settled();
  expect(target.textContent).toContain('Anima could not save the default check settings.');
  expect(target.textContent).not.toContain('secret hub text');
  expect(values(target)).toEqual(['240', '5', '45', '2500']);
  expect(save(target).getAttribute('aria-disabled')).toBe('false');
});

it('SET-002: Save keeps its label while busy, sends once, uses aria-disabled and is free again after', async () => {
  let release: (result: ReturnType<typeof ok>) => void = () => undefined;
  const answer = new Promise<ReturnType<typeof ok>>((resolve) => {
    release = resolve;
  });
  const target = await open({ CheckDefaults: ok({ checkDefaults: STORED }), SetCheckDefaults: answer });
  const button = save(target);
  const label = button.textContent;
  button.click();
  flushSync();
  expect(button.getAttribute('aria-disabled')).toBe('true');
  expect(button.disabled).toBe(false);
  expect(button.textContent).toBe(label);
  button.click();
  await settled();
  expect(hub.sent.filter((each) => each.operation === 'SetCheckDefaults')).toHaveLength(1);
  release(ok({ setCheckDefaults: SAVED }));
  await settled();
  expect(button.getAttribute('aria-disabled')).toBe('false');
});

it('D-26: a failed save frees Save for the next try', async () => {
  const target = await open({ CheckDefaults: ok({ checkDefaults: STORED }), SetCheckDefaults: failure('unreachable') });
  save(target).click();
  await settled();
  save(target).click();
  await settled();
  expect(hub.sent.filter((each) => each.operation === 'SetCheckDefaults')).toHaveLength(2);
});

it('MOT-007: the loading placeholder has the same rows, hints and message lines as the loaded section', async () => {
  const shape = (target: HTMLElement) => [...target.querySelectorAll('form > *')].map((each) => `${each.tagName}.${each.className}`);
  let release: (result: ReturnType<typeof ok>) => void = () => undefined;
  const answer = new Promise<ReturnType<typeof ok>>((resolve) => {
    release = resolve;
  });
  const target = await open({ CheckDefaults: answer });
  const before = shape(target);
  expect(values(target)).toEqual(['', '', '', '']);
  expect(LABELS.every((label) => field(target, label).disabled)).toBe(true);
  expect(save(target).getAttribute('aria-disabled')).toBe('true');
  release(ok({ checkDefaults: STORED }));
  await settled();
  expect(shape(target)).toEqual(before);
  expect(field(target, 'Interval (seconds)').disabled).toBe(false);
});

it('D-26: a failed load shows a fixed message with Try again, and Try again loads the values', async () => {
  const answers: Parameters<typeof fakeHub>[0] = { CheckDefaults: failure('unreachable') };
  const target = await open(answers);
  expect(target.querySelector('[role="alert"]:not(.problem)')?.textContent).toBe('Anima could not load the default check settings.');
  expect(save(target).getAttribute('aria-disabled')).toBe('true');
  answers.CheckDefaults = ok({ checkDefaults: STORED });
  const retry = [...target.querySelectorAll('button')].find((each) => each.textContent === 'Try again');
  retry?.click();
  await settled();
  expect(values(target)).toEqual(['120', '5', '45', '2500']);
  expect(target.textContent).not.toContain('could not load');
});
