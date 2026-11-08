// @vitest-environment jsdom
import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { print } from 'graphql';
import { flushSync } from 'svelte';
import { beforeEach, expect, it, vi } from 'vitest';
import type { Hub, HubResult } from '$lib/hub/hub';
import { FIXTURE_CHANNELS } from '$lib/notifications/notifications.fixture';
import { HTTP_SAVED } from '../check.fixture';
import { button, installFormHarness, settle, show, submit, type } from '../form.fixture';
import { ADDED_SENTENCE, LOAD_SENTENCE, PARTIAL_SENTENCE, SAVE_SENTENCE } from './draft.svelte';
import { RESEND_SENTENCE } from './channels';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown }));
vi.mock('$lib/hub/context', () => ({
  useHub: () => {
    if (mocks.hub === undefined) throw new Error('missing_context');
    return mocks.hub;
  },
}));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('../../../motion/engine.fixture')).fakeRun,
}));

installFormHarness();

const ok = <T,>(value: T): HubResult<T> => ({ ok: true, value });
const down: HubResult<never> = { ok: false, error: { code: 'UNAVAILABLE', message: 'secret hub text' } };
const CHANNELS = ok({ notificationChannels: FIXTURE_CHANNELS });
const OWN = ok({ check: { notifications: { channels: [{ id: FIXTURE_CHANNELS[0]?.id }], resendEveryBeats: 5 } } });
const stored = (ids: string[], resend: number) => ok({ setCheckNotifications: { channels: ids.map((id) => ({ id })), resendEveryBeats: resend } });
const refused = (...fields: { field: string; message: string }[]): HubResult<never> => ({ ok: false, error: { code: 'VALIDATION', message: 'Invalid', fields } });

interface Sent {
  readonly operation: string;
  readonly variables: unknown;
}

/** A hub that answers each operation from its queue (the last answer repeats), and records every request. */
function hubWith(answers: Record<string, HubResult<unknown>[]>): Sent[] {
  const sent: Sent[] = [];
  mocks.hub = {
    request: (document: TypedDocumentNode, variables: unknown) => {
      const operation = /(?:mutation|query) (\w+)/.exec(print(document))?.[1] ?? '';
      sent.push({ operation, variables });
      const queue = answers[operation] ?? [];
      return Promise.resolve((queue.length > 1 ? queue.shift() : queue[0]) ?? down);
    },
  } as unknown as Hub;
  return sent;
}

beforeEach(() => {
  mocks.hub = undefined;
});

/** Lets the hub round trips of a save finish: `settle` only covers a tick. */
const later = () => new Promise((done) => setTimeout(done, 0)).then(() => flushSync());
const section = () => document.querySelector<HTMLElement>('[data-section="notifications"]');
const boxes = () => [...(section()?.querySelectorAll<HTMLInputElement>('input[type="checkbox"]') ?? [])];
const box = (name: string) => boxes().find((input) => input.closest('label')?.textContent.includes(name)) ?? null;
const resend = () => document.querySelector<HTMLInputElement>('[data-section="notifications"] input:not([type="checkbox"])') as HTMLInputElement;
const status = () => section()?.querySelector('[role="status"]')?.textContent ?? '';
const pick = (name: string) => {
  box(name)?.click();
  flushSync();
};
const typeResend = (value: string) => {
  resend().value = value;
  resend().dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
};

it('NOT-003: a new check pre-selects the on-by-default channels and lists each with its kind', async () => {
  hubWith({ NotificationChannels: [CHANNELS] });
  show();
  await settle();
  expect(boxes()).toHaveLength(4);
  expect(box('phone-ntfy')?.checked).toBe(true);
  expect(box('ops-webhook')?.checked).toBe(false);
  expect(box('phone-ntfy')?.closest('label')?.textContent).toContain('ntfy');
  expect(resend().value).toBe('0');
});

it('NOT-003, NOT-005: an edit loads the check own channels and resend interval through their own document', async () => {
  const sent = hubWith({ NotificationChannels: [CHANNELS], CheckNotifications: [OWN] });
  show({ check: HTTP_SAVED });
  await settle();
  expect(sent.find((request) => request.operation === 'CheckNotifications')?.variables).toEqual({ id: 'chk-1' });
  expect(box('ops-webhook')?.checked).toBe(true);
  expect(box('phone-ntfy')?.checked).toBe(false);
  expect(resend().value).toBe('5');
});

it('NOT-003, NOT-005: an edit saves the changed channels first, then the check', async () => {
  const order: string[] = [];
  mocks.hub = {
    request: (document: TypedDocumentNode) => {
      const operation = /(?:mutation|query) (\w+)/.exec(print(document))?.[1] ?? '';
      if (operation === 'SetCheckNotifications') order.push('channels');
      return Promise.resolve(({ NotificationChannels: CHANNELS, CheckNotifications: OWN, SetCheckNotifications: stored([FIXTURE_CHANNELS[0]?.id ?? '', FIXTURE_CHANNELS[1]?.id ?? ''], 0) } as Record<string, HubResult<unknown>>)[operation] ?? down);
    },
  } as unknown as Hub;
  const done = vi.fn();
  const form = show({ check: HTTP_SAVED, onDone: done, onSave: () => { order.push('check'); return Promise.resolve(ok({})); } });
  await settle();
  pick('phone-ntfy');
  typeResend('0');
  await submit(form);
  await later();
  expect(order).toEqual(['channels', 'check']);
  expect(done).toHaveBeenCalledOnce();
});

it('NOT-003: untouched channels send no mutation', async () => {
  const sent = hubWith({ NotificationChannels: [CHANNELS], CheckNotifications: [OWN] });
  const form = show({ check: HTTP_SAVED });
  await settle();
  await submit(form);
  expect(sent.some((request) => request.operation === 'SetCheckNotifications')).toBe(false);
});

it('NOT-003: a new check saves its channels after the check, with the new check ID', async () => {
  const sent = hubWith({ NotificationChannels: [CHANNELS], SetCheckNotifications: [stored([FIXTURE_CHANNELS[1]?.id ?? ''], 3)] });
  const form = show({ onSave: () => Promise.resolve(ok({ id: 'new-1' })) });
  await settle();
  type('Name', 'ping');
  typeResend('3');
  await submit(form);
  expect(sent.find((request) => request.operation === 'SetCheckNotifications')?.variables).toEqual({
    checkId: 'new-1',
    input: { channelIds: [FIXTURE_CHANNELS[1]?.id], resendEveryBeats: 3 },
  });
});

it('NOT-005: a bad resend interval is refused before anything is sent, with focus on the input', async () => {
  const sent = hubWith({ NotificationChannels: [CHANNELS], CheckNotifications: [OWN] });
  const onSave = vi.fn(() => Promise.resolve(ok({})));
  const form = show({ check: HTTP_SAVED, onSave });
  await settle();
  typeResend('-2');
  await submit(form);
  expect(status()).toBe(RESEND_SENTENCE);
  expect(resend().getAttribute('aria-invalid')).toBe('true');
  expect(document.activeElement).toBe(resend());
  expect(onSave).not.toHaveBeenCalled();
  expect(sent.some((request) => request.operation === 'SetCheckNotifications')).toBe(false);
});

it('NOT-005: a hub refusal of the interval shows the sentence and focuses the input; the check is not saved', async () => {
  hubWith({ NotificationChannels: [CHANNELS], CheckNotifications: [OWN], SetCheckNotifications: [refused({ field: 'resendEveryBeats', message: 'too big' })] });
  const onSave = vi.fn(() => Promise.resolve(ok({})));
  const form = show({ check: HTTP_SAVED, onSave });
  await settle();
  typeResend('999999');
  await submit(form);
  await later();
  expect(status()).toBe(RESEND_SENTENCE);
  expect(document.activeElement).toBe(resend());
  expect(onSave).not.toHaveBeenCalled();
});

it('F-3: a failed channel save keeps the check unsaved and says so in a fixed sentence', async () => {
  hubWith({ NotificationChannels: [CHANNELS], CheckNotifications: [OWN], SetCheckNotifications: [down] });
  const onSave = vi.fn(() => Promise.resolve(ok({})));
  const form = show({ check: HTTP_SAVED, onSave });
  await settle();
  pick('phone-ntfy');
  await submit(form);
  await later();
  expect(status()).toBe(SAVE_SENTENCE);
  expect(document.body.textContent).not.toContain('secret hub text');
  expect(onSave).not.toHaveBeenCalled();
  expect(button('Save check').getAttribute('aria-disabled')).toBeNull();
});

it('F-3: "saved; check was not" shows only when the channels were really saved', async () => {
  hubWith({ NotificationChannels: [CHANNELS], CheckNotifications: [OWN], SetCheckNotifications: [stored([], 5)] });
  const refuse = () => Promise.resolve({ ok: false as const, error: { code: 'CONFLICT' as const, message: 'Name already used.' } });
  const form = show({ check: HTTP_SAVED, onSave: refuse });
  await settle();
  await submit(form);
  await later();
  expect(status()).toBe('');
  pick('ops-webhook');
  await submit(form);
  await later();
  expect(status()).toBe(PARTIAL_SENTENCE);
  await submit(form);
  await later();
  expect(status()).toBe(PARTIAL_SENTENCE);
});

it('F-3, R-085: a channel list that fails disables only this part; Try again keeps focus and the check still saves', async () => {
  const answers: Record<string, HubResult<unknown>[]> = { NotificationChannels: [down, CHANNELS] };
  const onSave = vi.fn(() => Promise.resolve(ok({})));
  const sent = hubWith(answers);
  const form = show({ onSave });
  await settle();
  expect(status()).toBe(LOAD_SENTENCE);
  expect(resend().disabled).toBe(true);
  type('Name', 'ping');
  await submit(form);
  await later();
  expect(onSave).toHaveBeenCalledOnce();
  expect(sent.some((request) => request.operation === 'SetCheckNotifications')).toBe(false);

  const retry = button('Try again');
  retry.focus();
  retry.click();
  flushSync();
  expect(button('Try again')).toBe(retry);
  expect(document.contains(retry)).toBe(true);
  expect(retry.getAttribute('aria-disabled')).toBe('true');
  expect(document.activeElement).toBe(retry);
  await later();
  expect(boxes()).toHaveLength(4);
  expect(document.activeElement).toBe(boxes()[0]);
  expect(status()).toBe('');
});

it('NOT-003: with no channels the section points to Settings', async () => {
  hubWith({ NotificationChannels: [ok({ notificationChannels: [] })] });
  show();
  await settle();
  expect(boxes()).toHaveLength(0);
  expect(section()?.textContent).toContain('See Settings → Notifications.');
});

it('MOT-007: the status line is mounted and the list box is one element in every state', async () => {
  let release: (result: HubResult<unknown>) => void = () => undefined;
  mocks.hub = {
    request: (document: TypedDocumentNode) => {
      if (!print(document).includes('NotificationChannels')) return Promise.resolve(down);
      return new Promise((done) => (release = done));
    },
  } as unknown as Hub;
  show();
  const list = section()?.querySelector('.list');
  expect(status()).toBe('');
  expect(section()?.querySelectorAll('.skeleton')).toHaveLength(3);
  expect(section()?.querySelector('[role="status"]')).not.toBeNull();
  release(CHANNELS);
  await settle();
  expect(section()?.querySelector('.list')).toBe(list);
});

it('R-085: without a hub above the form there is no Notifications part', () => {
  show();
  expect(section()).toBeNull();
});

it('NOT-003: Add check pressed while the list loads waits for it and sends the on-by-default channels', async () => {
  const sent: Sent[] = [];
  let release: (result: HubResult<unknown>) => void = () => undefined;
  mocks.hub = {
    request: (document: TypedDocumentNode, variables: unknown) => {
      const operation = /(?:mutation|query) (\w+)/.exec(print(document))?.[1] ?? '';
      sent.push({ operation, variables });
      if (operation === 'NotificationChannels') return new Promise((done) => (release = done));
      return Promise.resolve(operation === 'SetCheckNotifications' ? stored([FIXTURE_CHANNELS[1]?.id ?? ''], 0) : down);
    },
  } as unknown as Hub;
  const form = show({ onSave: () => Promise.resolve(ok({ id: 'new-1' })) });
  type('Name', 'ping');
  await submit(form);
  expect(sent.some((request) => request.operation === 'SetCheckNotifications')).toBe(false);
  release(CHANNELS);
  await later();
  expect(sent.find((request) => request.operation === 'SetCheckNotifications')?.variables).toEqual({
    checkId: 'new-1',
    input: { channelIds: [FIXTURE_CHANNELS[1]?.id], resendEveryBeats: 0 },
  });
});

it('NOT-003: a new check whose channels fail says so, stays open, and a second Save sends only the channels', async () => {
  const sent = hubWith({ NotificationChannels: [CHANNELS], SetCheckNotifications: [down, stored([FIXTURE_CHANNELS[1]?.id ?? ''], 0)] });
  const onSave = vi.fn(() => Promise.resolve(ok({ id: 'new-1' })));
  const done = vi.fn();
  const form = show({ onSave, onDone: done });
  await settle();
  type('Name', 'ping');
  await submit(form);
  await later();
  expect(status()).toBe(ADDED_SENTENCE);
  expect(done).not.toHaveBeenCalled();
  expect(button('Save channels')).toBeDefined();

  await submit(form);
  await later();
  expect(onSave).toHaveBeenCalledOnce();
  expect(sent.filter((request) => request.operation === 'SetCheckNotifications')).toHaveLength(2);
  expect(done).toHaveBeenCalledOnce();
});

it('NOT-003: a new check whose channels save calls onDone once', async () => {
  hubWith({ NotificationChannels: [CHANNELS], SetCheckNotifications: [stored([FIXTURE_CHANNELS[1]?.id ?? ''], 0)] });
  const done = vi.fn();
  const form = show({ onSave: () => Promise.resolve(ok({ id: 'new-1' })), onDone: done });
  await settle();
  type('Name', 'ping');
  await submit(form);
  await later();
  expect(done).toHaveBeenCalledOnce();
});
