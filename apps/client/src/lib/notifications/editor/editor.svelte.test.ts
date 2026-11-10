// @vitest-environment jsdom
// The channel editor and its secrets (F-8, LOG-007): what is sent, and that nothing typed outlives a save or a close.
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import type { NotificationChannel } from '../api';
import NotificationsSection from '../NotificationsSection.svelte';
import { button, choice, dialog, failure, fakeHub, FAKE_SECRET, FAKE_WEBHOOK_URL, field, fixtureChannel, holdAnimations, inDialog, loaded, NOWHERE, ok, pick, places, settled, typeInto } from '../notifications.fixture';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown }));

vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

const consoleSpies = (['log', 'info', 'warn', 'error', 'debug', 'trace'] as const).map((name) => vi.spyOn(console, name).mockImplementation(() => undefined));
let app: ReturnType<typeof mount> | undefined;
let hub: ReturnType<typeof fakeHub>;

const MADE: NotificationChannel = {
  id: '0190a000-0000-7000-8017-0000000000aa',
  name: 'new-hook',
  kind: 'WEBHOOK',
  onByDefault: false,
  settings: { __typename: 'WebhookSettings', origin: 'https://hooks.example.net', method: 'POST', headerNames: ['Authorization'] },
  template: null,
  lastDelivery: null,
};

beforeEach(() => {
  installAnimations();
  for (const spy of consoleSpies) spy.mockClear();
});

afterEach(async () => {
  if (app !== undefined) await unmount(app);
  app = undefined;
  document.body.replaceChildren();
  localStorage.clear();
  sessionStorage.clear();
});

async function open(answers: Parameters<typeof fakeHub>[0]): Promise<void> {
  hub = fakeHub(answers);
  mocks.hub = hub.hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(NotificationsSection, { target });
  flushSync();
  await settled();
}

/** Nothing is written to the console at all, so nothing written holds a secret (LOG-007). */
function expectSilentConsole(): void {
  expect(consoleSpies.flatMap((spy) => spy.mock.calls)).toEqual([]);
}

/** Opens Add channel and types a webhook whose URL and header value are secrets. */
async function typeNewWebhook(answers: Parameters<typeof fakeHub>[0]): Promise<void> {
  await open(loaded(answers));
  button('Add channel').click();
  flushSync();
  typeInto(field('Name') as HTMLInputElement, 'new-hook');
  typeInto(field('Webhook URL') as HTMLInputElement, FAKE_WEBHOOK_URL);
  button('Add header').click();
  flushSync();
  typeInto(field('Header 1 name') as HTMLInputElement, 'Authorization');
  typeInto(field('Header 1 value') as HTMLInputElement, FAKE_SECRET);
}

function sentInput(): unknown {
  const last = hub.sent.at(-1);
  return (last?.variables as { input?: unknown } | undefined)?.input;
}

it('F-8: an edited channel shows no stored secret: each secret field is empty and says what is stored', async () => {
  await open(loaded());
  button('Edit phone-ntfy').click();
  flushSync();
  expect([field('Topic').value, field('Access token').value]).toEqual(['', '']);
  expect(field('Access token').type).toBe('password');
  expect(dialog()?.textContent).toContain('Stored. Leave empty to keep it.');
  expect(choice('Remove the stored access token').checked).toBe(false);
});

it('F-8: secret fields carry no name (never in a URL), no spell check and no autofill', async () => {
  await typeNewWebhook({});
  for (const label of ['Webhook URL', 'Header 1 value']) {
    const control = field(label);
    expect([control.hasAttribute('name'), control.getAttribute('spellcheck'), control.getAttribute('autocomplete')]).toEqual([false, 'false', label === 'Webhook URL' ? 'off' : 'new-password']);
  }
});

it('F-8, NOT-002: an untouched secret is sent as null; the rest of the channel goes as it was', async () => {
  const channel = fixtureChannel('NTFY');
  await open(loaded({ UpdateNotificationChannel: ok({ updateNotificationChannel: channel }) }));
  button('Edit phone-ntfy').click();
  flushSync();
  typeInto(field('Name') as HTMLInputElement, 'phone');
  inDialog('Save').click();
  await settled();
  expect(hub.sent.at(-1)?.operation).toBe('UpdateNotificationChannel');
  expect(sentInput()).toEqual({ name: 'phone', onByDefault: true, settings: { ntfy: { server: 'https://ntfy.example.net', topic: null, token: null, priority: 3 } }, template: null });
});

it('F-8: a removed optional secret is sent as "" (ntfy token, SMTP password)', async () => {
  await open(loaded({ UpdateNotificationChannel: ok({ updateNotificationChannel: fixtureChannel('NTFY') }) }));
  button('Edit phone-ntfy').click();
  flushSync();
  pick(choice('Remove the stored access token'));
  expect(field('Access token').disabled).toBe(true);
  inDialog('Save').click();
  await settled();
  expect(sentInput()).toMatchObject({ settings: { ntfy: { token: '', topic: null } } });
  button('Edit ops-email').click();
  flushSync();
  pick(choice('Remove the stored password'));
  inDialog('Save').click();
  await settled();
  expect(sentInput()).toMatchObject({ settings: { email: { password: '', host: 'mail.example.net', to: ['ops@example.net'] } } });
});

it('F-8, LOG-007: after a save no secret is in the DOM, a field, storage, the URL or the console', async () => {
  await typeNewWebhook({ CreateNotificationChannel: ok({ createNotificationChannel: MADE }) });
  expect(places(FAKE_SECRET).values).toBe(2);
  holdAnimations();
  inDialog('Save').click();
  await settled();
  expect(sentInput()).toMatchObject({ settings: { webhook: { url: FAKE_WEBHOOK_URL, headers: [{ name: 'Authorization', value: FAKE_SECRET }] } } });
  // The dialog is still fading out, and holds nothing.
  expect(document.querySelector('.portal')).not.toBeNull();
  expect(places(FAKE_SECRET)).toEqual(NOWHERE);
  expect(document.querySelector('.list > li[aria-label="new-hook"] dd.mono')?.textContent).toBe('https://hooks.example.net');
  expectSilentConsole();
});

it.each([
  ['Cancel', () => { inDialog('Cancel').click(); }],
  ['Escape', () => { dialog()?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true })); }],
])('F-8, LOG-007: after %s no secret is in the DOM, a field, storage, the URL or the console', async (_, close) => {
  await typeNewWebhook({});
  close();
  flushSync();
  expect(places(FAKE_SECRET)).toEqual(NOWHERE);
  await settled();
  expect(dialog()).toBeNull();
  expect(hub.sent.map((request) => request.operation)).not.toContain('CreateNotificationChannel');
  expectSilentConsole();
});

it('F-8: leaving Settings with the editor open empties its fields at once, though the dialog is still fading', async () => {
  await typeNewWebhook({});
  holdAnimations();
  if (app !== undefined) void unmount(app, { outro: true });
  app = undefined;
  flushSync();
  expect(dialog()).not.toBeNull();
  expect(places(FAKE_SECRET)).toEqual(NOWHERE);
});

it('LOG-007: a refused save shows a fixed sentence, keeps the typed values for another try, and logs nothing', async () => {
  await typeNewWebhook({ CreateNotificationChannel: failure('INTERNAL', `target said ${FAKE_SECRET}`) });
  inDialog('Save').click();
  await settled();
  expect(dialog()?.querySelector('.failure')?.textContent).toBe('Channel not saved. Try again.');
  expect(places(FAKE_SECRET)).toMatchObject({ markup: 0, values: 2, storage: 0, url: 0 });
  expectSilentConsole();
});

it('D-21: a template with a placeholder outside the list is refused before anything is sent', async () => {
  await typeNewWebhook({});
  pick(choice('Own template'));
  typeInto(field('Title') as HTMLInputElement, '{device} on {hostname}');
  inDialog('Save').click();
  await settled();
  expect(hub.sent.map((request) => request.operation)).toEqual(['NotificationChannels', 'NotificationSettings']);
  const title = field('Title');
  expect([title.getAttribute('aria-invalid'), document.activeElement]).toEqual(['true', title]);
  expect(dialog()?.textContent).toContain('Not a placeholder: {hostname}.');
});

it('F-8: while the hub saves, Escape and Cancel are refused, so its answer is never lost unseen', async () => {
  let release: (value: unknown) => void = () => undefined;
  const answer = new Promise((resolve) => {
    release = resolve;
  });
  await typeNewWebhook({ CreateNotificationChannel: answer as Promise<never> });
  inDialog('Save').click();
  flushSync();
  expect(inDialog('Save').getAttribute('aria-busy')).toBe('true');
  dialog()?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
  inDialog('Cancel').click();
  await settled();
  expect(dialog()).not.toBeNull();
  release(ok({ createNotificationChannel: MADE }));
  await settled();
  expect(dialog()).toBeNull();
  expect(places(FAKE_SECRET)).toEqual(NOWHERE);
});

it('NOT-002, NOT-003: a new ntfy channel, on by default, goes out with its own settings input only', async () => {
  await open(loaded({ CreateNotificationChannel: ok({ createNotificationChannel: fixtureChannel('NTFY') }) }));
  button('Add channel').click();
  flushSync();
  typeInto(field('Name') as HTMLInputElement, 'phone');
  pick(choice('ntfy'));
  pick(choice('On by default'));
  typeInto(field('Topic') as HTMLInputElement, 'FAKE_SECRET_topic');
  pick(field('Priority') as HTMLSelectElement, '5');
  inDialog('Save').click();
  await settled();
  expect(sentInput()).toEqual({
    name: 'phone',
    onByDefault: true,
    settings: { ntfy: { server: 'https://ntfy.sh', topic: 'FAKE_SECRET_topic', token: null, priority: 5 } },
    template: null,
  });
});
