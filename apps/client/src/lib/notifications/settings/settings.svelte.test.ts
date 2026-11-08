// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import NotificationsSection from '../NotificationsSection.svelte';
import { button, choice, dialog, failure, fakeHub, field, FIXTURE_SETTINGS, inDialog, loaded, ok, pick, settled, typeInto, type Operation } from '../notifications.fixture';
import { DAYS_NOT_DESCENDING, DAYS_OUT_OF_RANGE, DAYS_TOO_MANY } from './days';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown }));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;
let hub: ReturnType<typeof fakeHub>;
let form: HTMLFormElement;

beforeEach(() => {
  installAnimations();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

async function open(answers: Parameters<typeof fakeHub>[0]): Promise<void> {
  hub = fakeHub(answers);
  mocks.hub = hub.hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(NotificationsSection, { target });
  flushSync();
  await settled();
  const found = target.querySelector<HTMLFormElement>('form.settings');
  if (found === null) throw new Error('no settings form');
  form = found;
}

const days = () => field('Certificate expiry days', form) as HTMLInputElement;
const title = () => field('Title', form) as HTMLInputElement;
const save = () => button('Save', form);
const problemOf = (control: HTMLElement) => document.getElementById(control.getAttribute('aria-describedby')?.split(' ').at(-1) ?? '')?.textContent;
const outcome = () => form.querySelector('.outcome')?.textContent;
const sentOperations = () => hub.sent.map((request) => request.operation);

it('NOT-006, NOT-007: the form shows the hub\'s expiry days and default template', async () => {
  await open(loaded());
  expect(days().value).toBe('21, 14, 7');
  expect([title().value, (field('Body', form) as HTMLTextAreaElement).value]).toEqual([FIXTURE_SETTINGS.template.title, FIXTURE_SETTINGS.template.body]);
  expect(form.textContent).toContain(`Placeholders: ${FIXTURE_SETTINGS.placeholders.join(' ')}`);
});

it.each([
  ['366, 14', DAYS_OUT_OF_RANGE],
  ['0', DAYS_OUT_OF_RANGE],
  ['90, 60, 30, 21, 14, 7', DAYS_TOO_MANY],
  ['7, 14, 21', DAYS_NOT_DESCENDING],
])('D-22: expiry days "%s" are refused before sending', async (typed, message) => {
  await open(loaded());
  typeInto(days(), typed);
  save().click();
  await settled();
  expect(problemOf(days())).toBe(message);
  expect(days().getAttribute('aria-invalid')).toBe('true');
  expect(sentOperations()).not.toContain('SetNotificationSettings');
});

it('D-21: a default template with a placeholder outside the list is refused before sending', async () => {
  await open(loaded());
  typeInto(title(), '{device} {ip}');
  save().click();
  await settled();
  expect(problemOf(title())).toMatch(/^Not a placeholder: \{ip\}\./);
  expect(sentOperations()).not.toContain('SetNotificationSettings');
});

it('D-21: the hub\'s placeholder list, not the D-21 one, decides what the settings form and the channel editor accept', async () => {
  const narrow = { ...FIXTURE_SETTINGS, placeholders: ['{device}'], template: { title: '{device}', body: '{device}' } };
  await open(loaded({ NotificationSettings: ok({ notificationSettings: narrow }) }));
  typeInto(title(), '{device} {check}');
  save().click();
  await settled();
  expect(problemOf(title())).toBe('Not a placeholder: {check}. Use only {device}.');
  button('Add channel').click();
  flushSync();
  pick(choice('Own template'));
  typeInto(field('Title') as HTMLInputElement, '{check}');
  inDialog('Save').click();
  await settled();
  expect(dialog()?.textContent).toContain('Not a placeholder: {check}. Use only {device}.');
  expect(sentOperations()).toEqual(['NotificationChannels', 'NotificationSettings']);
});

it('NOT-006, NOT-007: Save sends the days and the template, and says Saved', async () => {
  const savedSettings = { ...FIXTURE_SETTINGS, certificateDays: [30, 7] };
  await open(loaded({ SetNotificationSettings: ok({ setNotificationSettings: savedSettings }) }));
  typeInto(days(), '30,7');
  save().click();
  await settled();
  expect(hub.sent.at(-1)).toEqual({ operation: 'SetNotificationSettings', variables: { input: { certificateDays: [30, 7], template: FIXTURE_SETTINGS.template } } });
  expect([outcome(), days().value]).toEqual(['Saved.', '30, 7']);
});

it('NOT-006: Save pressed twice while the hub saves sends the settings once', async () => {
  let release: (value: unknown) => void = () => undefined;
  const answer = new Promise((resolve) => {
    release = resolve;
  });
  await open(loaded({ SetNotificationSettings: answer as Promise<never> }));
  save().click();
  save().click();
  flushSync();
  save().click();
  expect(save().getAttribute('aria-busy')).toBe('true');
  release(ok({ setNotificationSettings: FIXTURE_SETTINGS }));
  await settled();
  expect(sentOperations().filter((operation) => operation === 'SetNotificationSettings')).toHaveLength(1);
  expect(outcome()).toBe('Saved.');
});

it('LOG-007: a refused save says so in a fixed sentence and keeps what was typed', async () => {
  await open(loaded({ SetNotificationSettings: failure('VALIDATION', 'hub text', [{ field: 'input.certificateDays', message: 'hub text' }]) }));
  typeInto(days(), '30, 7');
  save().click();
  await settled();
  expect([outcome(), days().value]).toEqual(['The hub refused: Certificate expiry days.', '30, 7']);
});

it('F-3, MOT-007: while loading the fields wait, empty; a failed load offers Try again in the outcome\'s place', async () => {
  const answers: Partial<Record<Operation, unknown>> = loaded({ NotificationSettings: failure('UNAVAILABLE') });
  await open(answers as Parameters<typeof fakeHub>[0]);
  expect([days().value, days().matches(':disabled'), form.getAttribute('aria-busy')]).toEqual(['', true, 'true']);
  expect(form.querySelector('.note [role="alert"]')?.textContent).toBe('Anima could not load the notification settings.');
  expect(document.querySelectorAll('.list > li')).toHaveLength(4);
  answers.NotificationSettings = ok({ notificationSettings: FIXTURE_SETTINGS });
  button('Try again', form).click();
  await settled();
  expect([days().value, days().matches(':disabled'), form.getAttribute('aria-busy')]).toEqual(['21, 14, 7', false, 'false']);
});
