// @vitest-environment jsdom
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import { manualClock } from '$lib/motion/loading/clock.fixture';
import type { LoadingClock } from '$lib/motion/loading/loading';
import { threshold } from '$lib/motion/tokens';
import NotificationsSection from '../NotificationsSection.svelte';
import { button, failure, fakeHub, FAKE_SECRET, FIXTURE_CHANNELS, loaded, ok, settled, TEST_DELIVERY, type Operation } from '../notifications.fixture';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown }));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

let app: ReturnType<typeof mount> | undefined;
let hub: ReturnType<typeof fakeHub>;

beforeEach(() => {
  installAnimations();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
});

async function open(answers: Parameters<typeof fakeHub>[0], clock?: LoadingClock): Promise<HTMLElement> {
  hub = fakeHub(answers);
  mocks.hub = hub.hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(NotificationsSection, { target, props: { clock } });
  flushSync();
  await settled();
  return target;
}

function row(target: HTMLElement, name: string): HTMLElement {
  const found = target.querySelector<HTMLElement>(`.list > li[aria-label="${name}"]`);
  if (found === null) throw new Error(`no row ${name}`);
  return found;
}

/** The row's values by their labels; the last delivery's lines joined by " / ". */
function values(element: HTMLElement): Record<string, string> {
  const pairs = [...element.querySelectorAll('dl > div')].map((pair) => [
    pair.querySelector('dt')?.textContent.trim() ?? '',
    [...pair.querySelectorAll('dd')].map((dd) => dd.textContent.replace(/\s+/g, ' ').trim()).join(' / '),
  ]);
  return Object.fromEntries(pairs) as Record<string, string>;
}

function names(target: HTMLElement): (string | null)[] {
  return [...target.querySelectorAll('.list > li')].map((element) => element.getAttribute('aria-label'));
}

function pending(): { answer: Promise<never>; release: (result: unknown) => void } {
  let release: (result: unknown) => void = () => undefined;
  const answer = new Promise<never>((resolve) => {
    release = resolve as (result: unknown) => void;
  });
  return { answer, release };
}

it('NOT-002, D-17: one channel of each kind shows its name, kind, on by default and its target as an origin or host only', async () => {
  const target = await open(loaded());
  expect(names(target)).toEqual(['ops-webhook', 'phone-ntfy', 'ops-email', 'apprise-bridge']);
  const shown = names(target).map((name) => {
    const element = row(target, name ?? '');
    const { Target, 'On by default': onByDefault } = values(element);
    return [element.querySelector('.kind')?.textContent, Target, onByDefault];
  });
  expect(shown).toEqual([
    ['Webhook', 'https://hooks.example.net', 'No'],
    ['ntfy', 'ntfy.example.net', 'Yes'],
    ['Email', 'mail.example.net', 'No'],
    ['Apprise', 'https://apprise.example.net', 'No'],
  ]);
  // Email shows the server, never its recipients.
  expect(target.textContent).not.toContain('ops@example.net');
});

it('D-17, F-8: a target the hub sends with a path, query or credential shows as its origin only', async () => {
  const [webhook] = FIXTURE_CHANNELS;
  if (webhook === undefined) throw new Error('no fixture');
  const leaky = { ...webhook, settings: { __typename: 'WebhookSettings', origin: `https://u:${FAKE_SECRET}@hooks.example.net/x/${FAKE_SECRET}?t=${FAKE_SECRET}`, method: 'POST', headerNames: [] } };
  const target = await open(loaded({ NotificationChannels: ok({ notificationChannels: [leaky] }) }));
  expect(values(row(target, 'ops-webhook')).Target).toBe('https://hooks.example.net');
  expect(document.documentElement.outerHTML).not.toContain(FAKE_SECRET);
});

it('NOT-004: the last delivery shows its time, kind, Delivered or Failed in words, and the hub\'s fixed sentence', async () => {
  const target = await open(loaded());
  const webhook = row(target, 'ops-webhook');
  expect(values(webhook)['Last delivery']).toMatch(/· Domain expiry · Delivered \/ $/);
  expect(webhook.querySelector('time')?.getAttribute('datetime')).toBe('2026-11-06T12:00:00Z');
  expect(values(row(target, 'ops-email'))['Last delivery']).toMatch(/· Test · Failed \/ No answer in 8 seconds\.$/);
});

it('NOT-004: Send test sends the test mutation and shows the delivery\'s fixed sentence, never more', async () => {
  const target = await open(loaded({ SendTestNotification: ok({ sendTestNotification: TEST_DELIVERY }) }));
  const webhook = row(target, 'ops-webhook');
  button('Send test through ops-webhook').click();
  await settled();
  expect(hub.sent.at(-1)).toEqual({ operation: 'SendTestNotification', variables: { id: TEST_DELIVERY.channelId } });
  expect(webhook.querySelector('[role="status"]')?.textContent).toBe('Test failed. Target address not permitted.');
  expect(values(webhook)['Last delivery']).toMatch(/· Test · Failed \/ Target address not permitted\.$/);
});

it('NOT-004: a delivered test says so; a refused call shows a fixed sentence, not the hub\'s text', async () => {
  const answers = [ok({ sendTestNotification: { ...TEST_DELIVERY, ok: true, message: null } }), failure('INTERNAL', `reply ${FAKE_SECRET}`)];
  const target = await open(loaded({ SendTestNotification: () => answers.shift() ?? failure('unreachable') }));
  const status = () => row(target, 'ops-webhook').querySelector('[role="status"]')?.textContent;
  button('Send test through ops-webhook').click();
  await settled();
  expect(status()).toBe('Test delivered.');
  button('Send test through ops-webhook').click();
  await settled();
  expect(status()).toBe('Anima could not send the test. Try again.');
  expect(document.body.textContent).not.toContain(FAKE_SECRET);
});

it('NOT-004, MOT-007: while the hub tests, Send test keeps its label and focus, says it is busy and sends once', async () => {
  const { answer, release } = pending();
  const target = await open(loaded({ SendTestNotification: answer }));
  const send = button('Send test through ops-webhook');
  send.focus();
  send.click();
  flushSync();
  expect([send.textContent.trim(), send.getAttribute('aria-disabled'), send.getAttribute('aria-busy'), document.activeElement]).toEqual(['Send test', 'true', 'true', send]);
  expect(row(target, 'ops-webhook').querySelector('[role="status"]')?.textContent).toBe('Sending a test…');
  send.click();
  await settled();
  expect(hub.sent.filter((request) => request.operation === 'SendTestNotification')).toHaveLength(1);
  release(ok({ sendTestNotification: TEST_DELIVERY }));
  await settled();
  expect(send.getAttribute('aria-disabled')).toBe('false');
});

it('NOT-002: Delete asks first; Cancel sends nothing', async () => {
  const target = await open(loaded({ DeleteNotificationChannel: ok({ deleteNotificationChannel: 'x' }) }));
  button('Delete phone-ntfy').click();
  flushSync();
  const dialog = document.querySelector('[role="dialog"]');
  expect(dialog?.querySelector('h2')?.textContent).toBe('Delete phone-ntfy?');
  button('Cancel', dialog ?? document).click();
  await settled();
  expect(hub.sent.map((request) => request.operation)).not.toContain('DeleteNotificationChannel');
  expect(names(target)).toContain('phone-ntfy');
});

it('NOT-002, NAV-005: a confirmed delete sends the id, drops the row and moves focus to the row that took its place', async () => {
  const id = FIXTURE_CHANNELS[1]?.id ?? '';
  const target = await open(loaded({ DeleteNotificationChannel: ok({ deleteNotificationChannel: id }) }));
  button('Delete phone-ntfy').click();
  flushSync();
  button('Delete phone-ntfy', document.querySelector('[role="dialog"]') ?? document).click();
  await settled();
  await settled();
  expect(hub.sent.at(-1)).toEqual({ operation: 'DeleteNotificationChannel', variables: { id } });
  expect(names(target)).toEqual(['ops-webhook', 'ops-email', 'apprise-bridge']);
  expect(document.activeElement?.getAttribute('aria-label')).toBe('Delete ops-email');
});

it.each([
  ['the last row', FIXTURE_CHANNELS, 'apprise-bridge', 'Delete ops-email'],
  ['the only row', FIXTURE_CHANNELS.slice(0, 1), 'ops-webhook', 'Add channel'],
])('NAV-005: deleting %s moves focus to the row before it, else to Add channel', async (_, channels, name, focused) => {
  await open(loaded({ NotificationChannels: ok({ notificationChannels: channels }), DeleteNotificationChannel: ok({ deleteNotificationChannel: 'x' }) }));
  button(`Delete ${name}`).click();
  flushSync();
  button(`Delete ${name}`, document.querySelector('[role="dialog"]') ?? document).click();
  await settled();
  await settled();
  expect(document.activeElement?.getAttribute('aria-label') ?? document.activeElement?.textContent.trim()).toBe(focused);
});

it('LOG-007: a refused delete shows a fixed sentence in the dialog, not the hub\'s text', async () => {
  await open(loaded({ DeleteNotificationChannel: failure('INTERNAL', `hub ${FAKE_SECRET}`) }));
  button('Delete phone-ntfy').click();
  flushSync();
  button('Delete phone-ntfy', document.querySelector('[role="dialog"]') ?? document).click();
  await settled();
  expect(document.querySelector('[role="dialog"] [role="alert"]')?.textContent).toBe('The hub did not delete the channel. Try again.');
  expect(document.body.textContent).not.toContain(FAKE_SECRET);
});

it('NOT-002: while the hub deletes, Cancel and Escape are refused; its answer then drops the row and closes the dialog', async () => {
  let release: (value: unknown) => void = () => undefined;
  const answer = new Promise((resolve) => {
    release = resolve;
  });
  const target = await open(loaded({ DeleteNotificationChannel: answer as Promise<never> }));
  button('Delete phone-ntfy').click();
  flushSync();
  const dialog = () => document.querySelector('[role="dialog"]');
  button('Delete phone-ntfy', dialog() ?? document).click();
  flushSync();
  button('Cancel', dialog() ?? document).click();
  dialog()?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  await settled();
  expect(dialog()?.querySelector('h2')?.textContent).toBe('Delete phone-ntfy?');
  release(ok({ deleteNotificationChannel: FIXTURE_CHANNELS[1]?.id }));
  await settled();
  await settled();
  expect(names(target)).not.toContain('phone-ntfy');
  expect(dialog()).toBeNull();
});

it('MOT-007: after threshold-loading a skeleton row shows over the kept space, hidden from assistive technology', async () => {
  const time = manualClock();
  const { answer, release } = pending();
  const target = await open(loaded({ NotificationChannels: answer }), time.clock);
  const stack = target.querySelector('.stack');
  expect(stack?.getAttribute('aria-busy')).toBe('true');
  expect(target.querySelector('.sizer')?.getAttribute('aria-hidden')).toBe('true');
  time.advance(threshold.loading);
  flushSync();
  const skeleton = target.querySelector('.skeleton');
  expect([skeleton?.getAttribute('aria-hidden'), skeleton?.hasAttribute('inert'), skeleton?.parentElement]).toEqual(['true', true, stack]);
  release(ok({ notificationChannels: FIXTURE_CHANNELS }));
  await settled();
  time.advance(threshold.loadingMin);
  await settled();
  expect(target.querySelector('.skeleton')).toBeNull();
  expect(names(target)).toHaveLength(4);
});

it('F-3, MOT-007: a failed load shows its own message and Try again in the kept space; the settings still load', async () => {
  const answers: Partial<Record<Operation, unknown>> = loaded({ NotificationChannels: failure('UNAVAILABLE') });
  const target = await open(answers as Parameters<typeof fakeHub>[0]);
  const alert = target.querySelector('.stack [role="alert"]');
  expect(alert?.textContent).toBe('Anima could not load the notification channels.');
  expect(target.querySelector<HTMLInputElement>('input[type="text"]')?.value).toBe('21, 14, 7');
  answers.NotificationChannels = ok({ notificationChannels: [] });
  button('Try again').click();
  await settled();
  expect(target.querySelector('.stack p.quiet')?.textContent).toBe('No channels. Add one to hear when a check goes down or comes back up.');
});

it('MOT-007, NAV-005: Try again stays mounted and keeps focus while it retries and when it fails again, then focus goes to Add channel', async () => {
  const answers = loaded({ NotificationChannels: failure('UNAVAILABLE') });
  await open(answers);
  const again = button('Try again');
  again.focus();
  again.click();
  flushSync();
  expect([document.activeElement === again, again.getAttribute('aria-disabled'), again.getAttribute('aria-busy')]).toEqual([true, 'true', 'true']);
  again.click();
  await settled();
  expect(hub.sent.filter((each) => each.operation === 'NotificationChannels')).toHaveLength(2);
  await settled();
  expect([document.activeElement === again, again.isConnected, again.getAttribute('aria-disabled')]).toEqual([true, true, 'false']);
  answers.NotificationChannels = ok({ notificationChannels: FIXTURE_CHANNELS });
  again.click();
  await settled();
  await settled();
  expect(document.activeElement).toBe(button('Add channel'));
});
