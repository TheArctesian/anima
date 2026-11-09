// Test support: a hub that answers the notification operations from canned results and records what it was sent, the
// K6 channels in the shape K1 returns, and the synthetic values the tests use. Never a real secret (WP-50 checkpoint 4).
import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { print } from 'graphql';
import { flushSync } from 'svelte';
import { channels, deliveries, notificationSettings } from '$lib/fixtures/phase3';
import type { Hub, HubError, HubResult } from '$lib/hub/hub';
import type { Delivery, NotificationChannel, NotificationSettings } from './api';

/** A made-up secret. A test that finds it in the DOM, the console or storage has found a leak. */
export const FAKE_SECRET = 'FAKE_SECRET_7c1e';
/** A webhook URL that carries the secret in its path and query, as Discord and Slack URLs do (D-17). */
export const FAKE_WEBHOOK_URL = `https://hooks.example.net/services/${FAKE_SECRET}?token=${FAKE_SECRET}`;

const TYPENAMES = {
  WEBHOOK: 'WebhookSettings',
  NTFY: 'NtfySettings',
  EMAIL: 'EmailSettings',
  APPRISE: 'AppriseSettings',
} as const;

function lastDeliveryOf(id: string): Delivery | null {
  const own = deliveries.filter((delivery) => delivery.channelId === id).toSorted((a, b) => Date.parse(b.at) - Date.parse(a.at));
  const [last] = own;
  return last === undefined ? null : { id: last.id, channelId: last.channelId, kind: last.kind, at: last.at, ok: last.ok, reason: last.reason, code: last.code };
}

/** The K6 channels, one of each kind, each with its latest K6 delivery. */
export const FIXTURE_CHANNELS: readonly NotificationChannel[] = channels.map(
  (channel) =>
    ({
      id: channel.id,
      name: channel.name,
      kind: channel.kind,
      onByDefault: channel.onByDefault,
      settings: { __typename: TYPENAMES[channel.kind], ...channel.settings },
      template: channel.template,
      lastDelivery: lastDeliveryOf(channel.id),
    }) as NotificationChannel,
);

export function fixtureChannel(kind: NotificationChannel['kind']): NotificationChannel {
  const found = FIXTURE_CHANNELS.find((channel) => channel.kind === kind);
  if (found === undefined) throw new Error(`no ${kind} channel`);
  return found;
}

/** An Apprise channel that sends with a configuration key on the server, which K6 does not have (CR-14.2). */
export const CONFIG_KEY_CHANNEL: NotificationChannel = {
  ...fixtureChannel('APPRISE'),
  id: '0190a000-0000-7000-8017-0000000000ab',
  name: 'apprise-keyed',
  settings: { __typename: 'AppriseSettings', server: 'https://apprise.example.net', mode: 'CONFIG_KEY', tags: [] },
};

/** An ntfy channel whose server, as a hub before CR-14.7 could hold it, carries a user name and password. */
export const DIRTY_SERVER_CHANNEL: NotificationChannel = {
  ...fixtureChannel('NTFY'),
  id: '0190a000-0000-7000-8017-0000000000ac',
  name: 'dirty-ntfy',
  settings: { __typename: 'NtfySettings', server: `https://ops:${FAKE_SECRET}@ntfy.example.net`, priority: 3, tokenStored: true },
};

/** The same for an Apprise channel (CR-14.7). */
export const DIRTY_APPRISE_CHANNEL: NotificationChannel = {
  ...fixtureChannel('APPRISE'),
  id: '0190a000-0000-7000-8017-0000000000ad',
  name: 'dirty-apprise',
  settings: { __typename: 'AppriseSettings', server: `https://ops:${FAKE_SECRET}@apprise.example.net`, mode: 'STATELESS_URLS', tags: [] },
};

/** The K6 `notificationSettings`. */
export const FIXTURE_SETTINGS: NotificationSettings = notificationSettings;

export const TEST_DELIVERY: Delivery = {
  id: '0190a000-0000-7000-8018-0000000000aa',
  channelId: '0190a000-0000-7000-8017-000000000001',
  kind: 'TEST',
  at: '2026-11-07T12:00:00Z',
  ok: false,
  reason: 'TARGET_NOT_PERMITTED',
  code: null,
};

export type Operation =
  | 'NotificationChannels'
  | 'NotificationSettings'
  | 'CreateNotificationChannel'
  | 'UpdateNotificationChannel'
  | 'DeleteNotificationChannel'
  | 'SendTestNotification'
  | 'SetNotificationSettings';

interface SentRequest {
  readonly operation: string;
  readonly variables: unknown;
}

export function ok<T>(value: T): HubResult<T> {
  return { ok: true, value };
}

export function failure(code: HubError['code'], message = 'hub text', fields: readonly { field: string; message: string }[] = []): HubResult<never> {
  return { ok: false, error: { code, message, fields } as HubError };
}

type Answer = HubResult<unknown> | Promise<HubResult<unknown>> | (() => HubResult<unknown> | Promise<HubResult<unknown>>);

/** `answers` maps an operation to its result, or to a function giving one per call; an operation without one is `unreachable`. */
export function fakeHub(answers: Partial<Record<Operation, Answer>>): { hub: Hub; sent: SentRequest[] } {
  const sent: SentRequest[] = [];
  const hub = {
    request: (document: TypedDocumentNode, variables: unknown) => {
      const operation = /(?:mutation|query) (\w+)/.exec(print(document))?.[1] ?? '';
      sent.push({ operation, variables });
      const answer = answers[operation as Operation];
      return Promise.resolve(typeof answer === 'function' ? answer() : (answer ?? failure('unreachable')));
    },
  } as unknown as Hub;
  return { hub, sent };
}

/** The usual answers: the K6 channels and settings. */
export function loaded(more: Partial<Record<Operation, Answer>> = {}): Partial<Record<Operation, Answer>> {
  return { NotificationChannels: ok({ notificationChannels: FIXTURE_CHANNELS }), NotificationSettings: ok({ notificationSettings: FIXTURE_SETTINGS }), ...more };
}

export async function settled(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

/** The button whose accessible name, or else its text, is `label`. */
export function button(label: string, within: ParentNode = document): HTMLButtonElement {
  const found = [...within.querySelectorAll('button')].find((candidate) => (candidate.getAttribute('aria-label') ?? candidate.textContent.trim()) === label);
  if (found === undefined) throw new Error(`no button ${label}`);
  return found;
}

/** The field labelled `label` in the open dialog, or in `within`. */
export function field(label: string, within?: ParentNode): HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement {
  const scope = within ?? document.querySelector('[role="dialog"]') ?? document;
  const found = [...scope.querySelectorAll('label')].find((candidate) => candidate.textContent.trim() === label);
  const control = found?.htmlFor === undefined || found.htmlFor === '' ? null : document.getElementById(found.htmlFor);
  if (!(control instanceof HTMLInputElement || control instanceof HTMLTextAreaElement || control instanceof HTMLSelectElement)) throw new Error(`no field ${label}`);
  return control;
}

/** The checkbox or radio whose label starts with `label`, in the open dialog. */
export function choice(label: string): HTMLInputElement {
  const scope = document.querySelector('[role="dialog"]') ?? document;
  const found = [...scope.querySelectorAll('label')].find((candidate) => candidate.textContent.trim().startsWith(label));
  const input = found?.querySelector('input');
  if (!(input instanceof HTMLInputElement)) throw new Error(`no choice ${label}`);
  return input;
}

/** Types into a field the way the browser reports it. */
export function typeInto(control: HTMLInputElement | HTMLTextAreaElement, value: string): void {
  control.value = value;
  control.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
}

export function pick(control: HTMLSelectElement | HTMLInputElement, value?: string): void {
  if (control instanceof HTMLInputElement) control.click();
  else {
    control.value = value ?? '';
    control.dispatchEvent(new Event('change', { bubbles: true }));
  }
  flushSync();
}

export function dialog(): HTMLElement | null {
  return document.querySelector('[role="dialog"]');
}

/** Fades that never finish, so a test sees the document as it is during a fade. */
export function holdAnimations(): void {
  Object.defineProperty(Element.prototype, 'animate', {
    configurable: true,
    value: () => ({ playState: 'running', currentTime: 0, cancel: () => undefined, pause: () => undefined, onfinish: null }),
  });
}

/** A button of the open dialog: the page has its own Save. */
export function inDialog(label: string): HTMLButtonElement {
  const scope = dialog();
  if (scope === null) throw new Error('no dialog');
  return button(label, scope.closest('.portal') ?? scope);
}

/** Whether a node keeps `text` in a property of its own, as a cache keyed by a symbol would. */
function nodeKeeps(element: Element, text: string): boolean {
  const own = element as unknown as Record<PropertyKey, unknown>;
  return [...Object.getOwnPropertyNames(element), ...Object.getOwnPropertySymbols(element)].some((name) => {
    try {
      return JSON.stringify(own[name])?.includes(text) === true;
    } catch {
      // A value JSON cannot write (a cycle) is a reference to an object, not a copy of the text.
      return false;
    }
  });
}

/** Every place a secret could linger: the markup, field values, fields' own properties, storage, the URL. */
export function places(text: string): Record<string, number> {
  const fields = [...document.querySelectorAll('input, textarea, select')];
  const stored = [localStorage, sessionStorage].flatMap((storage) => Object.keys(storage).map((key) => `${key}=${storage.getItem(key) ?? ''}`));
  return {
    markup: document.documentElement.outerHTML.split(text).length - 1,
    values: fields.filter((element) => (element as HTMLInputElement).value.includes(text)).length,
    kept: fields.filter((element) => nodeKeeps(element, text)).length,
    storage: stored.filter((entry) => entry.includes(text)).length,
    url: window.location.href.includes(text) ? 1 : 0,
  };
}

export const NOWHERE = { markup: 0, values: 0, kept: 0, storage: 0, url: 0 };
