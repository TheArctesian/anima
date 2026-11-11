// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, expect, it, vi } from 'vitest';
import RetentionSection from './RetentionSection.svelte';
import { failure, fakeHub, FIXTURE_PERIODS, ok, settled } from './retention.fixture';

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

async function open(answers: Parameters<typeof fakeHub>[0], wait = true): Promise<HTMLElement> {
  hub = fakeHub(answers);
  mocks.hub = hub.hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(RetentionSection, { target });
  flushSync();
  if (wait) await settled();
  return target;
}

function row(target: HTMLElement, label: string): HTMLFormElement {
  const found = target.querySelector<HTMLFormElement>(`form[aria-label="${label}"]`);
  if (found === null) throw new Error(`no row ${label}`);
  return found;
}

function input(target: HTMLElement, label: string): HTMLInputElement {
  const found = row(target, label).querySelector('input');
  if (found === null) throw new Error(`no input ${label}`);
  return found;
}

function type(field: HTMLInputElement, value: string): void {
  field.value = value;
  field.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
}

async function save(target: HTMLElement, label: string): Promise<void> {
  row(target, label).querySelector('button')?.click();
  await settled();
}

const sentSets = () => hub.sent.filter((request) => request.operation === 'SetRetention');
const problemOf = (target: HTMLElement, label: string) => row(target, label).querySelector('[role="alert"]')?.textContent;

const LIST = { Retention: ok({ retention: FIXTURE_PERIODS }) };

it('HUB-005, D-25: each subject has a row showing its days, bounds and default from the hub', async () => {
  const target = await open(LIST);
  const labels = [...target.querySelectorAll('form')].map((form) => form.getAttribute('aria-label'));
  expect(labels).toEqual(['Check results', 'Metrics', 'Events', 'Session recordings', 'Notification deliveries']);
  const events = row(target, 'Events');
  const field = events.querySelector('input');
  expect(field?.value).toBe('90');
  expect(field?.min).toBe('30');
  expect(field?.max).toBe('365');
  expect(events.querySelector('.hint')?.textContent.replace(/\s+/g, ' ').trim()).toBe('Keeps 90 days. 30 to 365 days, default 90');
  expect(field?.getAttribute('aria-describedby')).toContain(events.querySelector('.hint')?.id);
});

it('HUB-005: a value outside the bounds is refused before sending, with the message tied to the input', async () => {
  const target = await open(LIST);
  const field = input(target, 'Events');
  type(field, '10');
  await save(target, 'Events');
  expect(sentSets()).toHaveLength(0);
  expect(problemOf(target, 'Events')).toBe('Enter a number of days from 30 to 365.');
  expect(field.getAttribute('aria-invalid')).toBe('true');
  expect(field.getAttribute('aria-describedby')).toContain(row(target, 'Events').querySelector('[role="alert"]')?.id);
  expect(field.value).toBe('10');
});

it('HUB-005: a value that is not a whole number is refused before sending', async () => {
  const target = await open(LIST);
  type(input(target, 'Metrics'), '2.5');
  await save(target, 'Metrics');
  type(input(target, 'Metrics'), '');
  await save(target, 'Metrics');
  expect(sentSets()).toHaveLength(0);
  expect(problemOf(target, 'Metrics')).toBe('Enter a whole number of days.');
});

it('HUB-005, D-25: Save sends setRetention with the subject and days and the row shows the period the hub returns', async () => {
  const returned = { subject: 'EVENTS', days: 120, minDays: 30, maxDays: 365, defaultDays: 90 };
  const target = await open({ ...LIST, SetRetention: ok({ setRetention: returned }) });
  type(input(target, 'Events'), '120');
  await save(target, 'Events');
  expect(sentSets()).toEqual([{ operation: 'SetRetention', variables: { subject: 'EVENTS', days: 120 } }]);
  expect(input(target, 'Events').value).toBe('120');
  expect(row(target, 'Events').querySelector('.hint')?.textContent).toContain('Keeps 120 days.');
  expect(problemOf(target, 'Events')).toBe('');
});

it('HUB-005: a failed save shows a fixed message, not the hub text, and keeps what the operator typed', async () => {
  const target = await open({ ...LIST, SetRetention: failure('INTERNAL', 'secret hub text') });
  const field = input(target, 'Events');
  type(field, '120');
  await save(target, 'Events');
  expect(problemOf(target, 'Events')).toBe('Anima could not save the events period.');
  expect(target.textContent).not.toContain('secret hub text');
  expect(field.value).toBe('120');
  expect(row(target, 'Events').querySelector('.hint')?.textContent).toContain('Keeps 90 days.');
});

it('HUB-005: the note that changes apply at the next daily run is shown', async () => {
  const target = await open(LIST);
  expect(target.textContent).toContain('Applied at the next daily run.');
});

it('HUB-005: a failed load shows a fixed message and Try again loads the periods', async () => {
  const target = await open({ Retention: failure('unreachable', 'secret hub text') });
  expect(target.querySelector('.note [role="alert"]')?.textContent).toBe('Could not load the periods.');
  expect(target.textContent).not.toContain('secret hub text');
  hub.hub.request = (() => Promise.resolve(ok({ retention: FIXTURE_PERIODS }))) as typeof hub.hub.request;
  [...target.querySelectorAll('button')].find((each) => each.textContent === 'Try again')?.click();
  await settled();
  expect(input(target, 'Events').value).toBe('90');
  expect(target.querySelector('.note [role="alert"]')).toBeNull();
});

it('MOT-007, MV-005: while loading the five rows are already laid out with their controls waiting, and the same rows stay when the hub answers', async () => {
  let release: (result: ReturnType<typeof ok>) => void = () => undefined;
  const pending = new Promise<ReturnType<typeof ok>>((resolve) => {
    release = resolve;
  });
  const target = await open({ Retention: pending }, false);
  const shape = () => [...target.querySelectorAll('form')].map((form) => [...form.children].map((child) => child.tagName + '.' + child.className).join(' '));
  const before = shape();
  expect(before).toHaveLength(5);
  expect(target.querySelectorAll('input:disabled')).toHaveLength(5);
  expect(target.querySelectorAll('.hint.waiting')).toHaveLength(5);
  release(ok({ retention: FIXTURE_PERIODS }));
  await settled();
  expect(shape().map((each) => each.replace(' waiting', ''))).toEqual(before.map((each) => each.replace(' waiting', '')));
  expect(target.querySelectorAll('input:disabled')).toHaveLength(0);
});

it('MOT-007: Save keeps its label and place while the hub answers, and keeps focus', async () => {
  let release: (result: ReturnType<typeof ok>) => void = () => undefined;
  const pending = new Promise<ReturnType<typeof ok>>((resolve) => {
    release = resolve;
  });
  const target = await open({ ...LIST, SetRetention: pending });
  const button = row(target, 'Events').querySelector('button');
  button?.focus();
  type(input(target, 'Events'), '120');
  button?.click();
  flushSync();
  // Under threshold-loading the label is still Save (the pending label waits, so it never flashes).
  expect(button?.querySelector('[aria-hidden="false"]')?.textContent).toBe('Save');
  expect(button?.getAttribute('aria-disabled')).toBe('true');
  button?.click();
  expect(sentSets()).toHaveLength(1);
  expect(document.activeElement).toBe(button);
  release(ok({ setRetention: { ...FIXTURE_PERIODS[2], days: 120 } }));
  await settled();
  expect(document.activeElement).toBe(button);
});

it('HUB-005: after a save resolves the row can save again', async () => {
  const target = await open({ ...LIST, SetRetention: ok({ setRetention: { ...FIXTURE_PERIODS[2], days: 120 } }) });
  type(input(target, 'Events'), '120');
  await save(target, 'Events');
  type(input(target, 'Events'), '150');
  await save(target, 'Events');
  expect(sentSets().map((request) => request.variables)).toEqual([
    { subject: 'EVENTS', days: 120 },
    { subject: 'EVENTS', days: 150 },
  ]);
});

it('HUB-005, D-25: saving another subject sends that subject and leaves the other rows as they were', async () => {
  const returned = { ...FIXTURE_PERIODS[3], days: 60 };
  const target = await open({ ...LIST, SetRetention: ok({ setRetention: returned }) });
  type(input(target, 'Session recordings'), '60');
  await save(target, 'Session recordings');
  expect(sentSets().map((request) => request.variables)).toEqual([{ subject: 'RECORDINGS', days: 60 }]);
  expect(input(target, 'Session recordings').value).toBe('60');
  const others = ['Check results', 'Metrics', 'Events', 'Notification deliveries'];
  expect(others.map((label) => input(target, label).value)).toEqual(['30', '30', '90', '30']);
});

it('HUB-005: a valid save after a refusal clears the refusal', async () => {
  const target = await open({ ...LIST, SetRetention: ok({ setRetention: { ...FIXTURE_PERIODS[2], days: 120 } }) });
  type(input(target, 'Events'), '10');
  await save(target, 'Events');
  expect(problemOf(target, 'Events')).not.toBe('');
  type(input(target, 'Events'), '120');
  await save(target, 'Events');
  expect(sentSets()).toHaveLength(1);
  expect(problemOf(target, 'Events')).toBe('');
});

it('HUB-005: editing the field clears the refusal; the loading hint is not read as a description', async () => {
  const target = await open(LIST);
  type(input(target, 'Events'), '10');
  await save(target, 'Events');
  type(input(target, 'Events'), '11');
  expect(problemOf(target, 'Events')).toBe('');
  expect(target.querySelectorAll('form')).toHaveLength(5);
  expect(row(target, 'Events').getAttribute('role')).toBe('group');
});

it('HUB-005: a save that works after a failed one, with no edit between, clears the failure message', async () => {
  const target = await open({ ...LIST, SetRetention: failure('INTERNAL') });
  type(input(target, 'Events'), '120');
  await save(target, 'Events');
  expect(problemOf(target, 'Events')).not.toBe('');
  hub.hub.request = (() => Promise.resolve(ok({ setRetention: { ...FIXTURE_PERIODS[2], days: 120 } }))) as typeof hub.hub.request;
  await save(target, 'Events');
  expect(problemOf(target, 'Events')).toBe('');
});

it('MOT-007, MV-005: the loading and loaded hints have the same two-line structure, and each row reserves its message block', async () => {
  let release: (result: ReturnType<typeof ok>) => void = () => undefined;
  const pending = new Promise<ReturnType<typeof ok>>((resolve) => {
    release = resolve;
  });
  const target = await open({ Retention: pending }, false);
  const lines = () => [...target.querySelectorAll('form')].map((form) => form.querySelectorAll('.hint > span').length);
  expect(lines()).toEqual([2, 2, 2, 2, 2]);
  expect(target.querySelectorAll('.problem')).toHaveLength(5);
  release(ok({ retention: FIXTURE_PERIODS }));
  await settled();
  expect(lines()).toEqual([2, 2, 2, 2, 2]);
  expect(target.querySelectorAll('.problem')).toHaveLength(5);
  expect(row(target, 'Events').querySelectorAll('.hint > span')[0]?.textContent).toBe('Keeps 90 days.');
  expect(row(target, 'Events').querySelectorAll('.hint > span')[1]?.textContent).toBe('30 to 365 days, default 90');
});

it('MOT-007: the note block is present in the loaded and the failed state', async () => {
  const loaded = await open(LIST);
  expect(loaded.querySelectorAll('.note')).toHaveLength(1);
  await unmount(app as ReturnType<typeof mount>);
  app = undefined;
  document.body.replaceChildren();
  const failed = await open({ Retention: failure('unreachable') });
  expect(failed.querySelectorAll('.note')).toHaveLength(1);
});

it('MOT-007, NAV-005: Try again stays mounted and keeps focus while it retries and when it fails again, then focus goes to the first period', async () => {
  const answers: Parameters<typeof fakeHub>[0] = { Retention: failure('unreachable') };
  const target = await open(answers);
  const again = [...target.querySelectorAll('button')].find((each) => each.textContent === 'Try again');
  if (again === undefined) throw new Error('no Try again');
  again.focus();
  again.click();
  flushSync();
  expect([document.activeElement === again, again.getAttribute('aria-disabled'), again.getAttribute('aria-busy')]).toEqual([true, 'true', 'true']);
  again.click();
  await settled();
  expect(hub.sent.filter((each) => each.operation === 'Retention')).toHaveLength(2);
  await settled();
  expect([document.activeElement === again, again.isConnected, again.getAttribute('aria-disabled')]).toEqual([true, true, 'false']);
  answers.Retention = ok({ retention: FIXTURE_PERIODS });
  again.click();
  await settled();
  await settled();
  expect(document.activeElement).toBe(target.querySelector('input'));
});
