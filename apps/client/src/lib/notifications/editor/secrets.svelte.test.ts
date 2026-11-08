// @vitest-environment jsdom
// The editor's secrets against a new target, a dirty server, Remove, a double submit and a fading dialog
// (CR-14.1, CR-14.7, F-8, LOG-007). The secret rules themselves are in target-change.test.ts.
import { flushSync, mount, unmount } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import { installAnimations } from '$lib/forms/animations.fixture';
import type { NotificationChannel } from '../api';
import NotificationsSection from '../NotificationsSection.svelte';
import {
  button,
  choice,
  dialog,
  DIRTY_APPRISE_CHANNEL,
  DIRTY_SERVER_CHANNEL,
  fakeHub,
  FAKE_SECRET,
  field,
  fixtureChannel,
  FIXTURE_CHANNELS,
  holdAnimations,
  inDialog,
  loaded,
  NOWHERE,
  ok,
  pick,
  places,
  settled,
  typeInto,
} from '../notifications.fixture';
import { AGAIN, AGAIN_OR_REMOVE, APPRISE_URLS_WRONG, HEADERS_AGAIN } from './input';

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

async function open(answers: Parameters<typeof fakeHub>[0]): Promise<void> {
  hub = fakeHub(answers);
  mocks.hub = hub.hub;
  const target = document.createElement('div');
  document.body.append(target);
  app = mount(NotificationsSection, { target });
  flushSync();
  await settled();
}

async function edit(name: string, channels: readonly NotificationChannel[] = FIXTURE_CHANNELS): Promise<void> {
  await open({ ...loaded(), NotificationChannels: ok({ notificationChannels: channels }), UpdateNotificationChannel: ok({ updateNotificationChannel: fixtureChannel('NTFY') }) });
  button(`Edit ${name}`).click();
  flushSync();
}

function operations(): string[] {
  return hub.sent.map((request) => request.operation);
}

function sentInput(): unknown {
  return (hub.sent.at(-1)?.variables as { input?: unknown } | undefined)?.input;
}

it('CR-14.1, D-17: Server moved to another host with the stored topic and token untouched is refused, and nothing is sent', async () => {
  await edit('phone-ntfy');
  typeInto(field('Server') as HTMLInputElement, 'https://ntfy.attacker.example');
  inDialog('Save').click();
  await settled();
  expect(operations()).not.toContain('UpdateNotificationChannel');
  expect(dialog()?.textContent).toContain(AGAIN);
  expect(dialog()?.textContent).toContain(AGAIN_OR_REMOVE);
  expect([field('Topic').getAttribute('aria-invalid'), document.activeElement]).toEqual(['true', field('Topic')]);
});

it('CR-14.1: the moved server is saved once the topic and the token are typed again', async () => {
  await edit('phone-ntfy');
  typeInto(field('Server') as HTMLInputElement, 'https://ntfy.other.example');
  typeInto(field('Topic') as HTMLInputElement, 'FAKE_SECRET_topic');
  typeInto(field('Access token') as HTMLInputElement, FAKE_SECRET);
  inDialog('Save').click();
  await settled();
  expect(sentInput()).toMatchObject({ settings: { ntfy: { server: 'https://ntfy.other.example', topic: 'FAKE_SECRET_topic', token: FAKE_SECRET } } });
  expect(places(FAKE_SECRET)).toEqual(NOWHERE);
});

it.each([
  ['ops-webhook', 'Webhook URL', `https://hooks.attacker.example/${FAKE_SECRET}`, HEADERS_AGAIN],
  ['ops-email', 'SMTP host', 'smtp.attacker.example', AGAIN_OR_REMOVE],
  ['apprise-bridge', 'Apprise server', 'https://apprise.attacker.example', APPRISE_URLS_WRONG],
])('CR-14.1: %s moved to a new target with its stored secrets untouched is refused, and nothing is sent', async (name, label, target, sentence) => {
  await edit(name);
  typeInto(field(label) as HTMLInputElement, target);
  inDialog('Save').click();
  await settled();
  expect(operations()).not.toContain('UpdateNotificationChannel');
  expect(dialog()?.textContent).toContain(sentence);
});

it.each([
  [DIRTY_SERVER_CHANNEL, 'Server', 'https://ntfy.example.net'],
  [DIRTY_APPRISE_CHANNEL, 'Apprise server', 'https://apprise.example.net'],
])('CR-14.7, F-8: a server the hub holds with a user name and password shows as its origin only, in the list and the editor (%#)', async (channel, label, origin) => {
  await edit(channel.name, [...FIXTURE_CHANNELS, channel]);
  expect(field(label).value).toBe(origin);
  expect(places(FAKE_SECRET)).toEqual(NOWHERE);
});

it('F-8: a token typed and then removed is sent as "", and what was typed goes nowhere', async () => {
  await edit('phone-ntfy');
  typeInto(field('Access token') as HTMLInputElement, FAKE_SECRET);
  pick(choice('Remove the stored access token'));
  expect(field('Access token').value).toBe('');
  inDialog('Save').click();
  await settled();
  expect(sentInput()).toMatchObject({ settings: { ntfy: { token: '' } } });
  expect(JSON.stringify(hub.sent)).not.toContain(FAKE_SECRET);
});

it('F-8: header values typed and then given up with Replace unticked are dropped, and the stored headers kept', async () => {
  await edit('ops-webhook');
  pick(choice('Replace the headers'));
  typeInto(field('Header 1 value') as HTMLInputElement, FAKE_SECRET);
  pick(choice('Replace the headers'));
  expect(places(FAKE_SECRET)).toEqual(NOWHERE);
  inDialog('Save').click();
  await settled();
  expect(sentInput()).toMatchObject({ settings: { webhook: { url: null, headers: null } } });
  expect(JSON.stringify(hub.sent)).not.toContain(FAKE_SECRET);
});

it('F-8: leaving Settings while Apprise URLs are typed empties the textarea at once, though the dialog is still fading', async () => {
  await edit('apprise-bridge');
  typeInto(field('Apprise URLs') as HTMLTextAreaElement, `discord://${FAKE_SECRET}`);
  holdAnimations();
  if (app !== undefined) void unmount(app, { outro: true });
  app = undefined;
  flushSync();
  expect(dialog()?.querySelector('textarea')).not.toBeNull();
  expect(places(FAKE_SECRET)).toEqual(NOWHERE);
});

it('NOT-002: Save pressed twice while the hub saves sends one create', async () => {
  let release: (value: unknown) => void = () => undefined;
  const answer = new Promise((resolve) => {
    release = resolve;
  });
  await open(loaded({ CreateNotificationChannel: answer as Promise<never> }));
  button('Add channel').click();
  flushSync();
  typeInto(field('Name') as HTMLInputElement, 'phone');
  pick(choice('ntfy'));
  typeInto(field('Topic') as HTMLInputElement, 'FAKE_SECRET_topic');
  inDialog('Save').click();
  inDialog('Save').click();
  flushSync();
  inDialog('Save').click();
  release(ok({ createNotificationChannel: fixtureChannel('NTFY') }));
  await settled();
  expect(operations().filter((operation) => operation === 'CreateNotificationChannel')).toHaveLength(1);
});
