// @vitest-environment jsdom
import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { print } from 'graphql';
import { flushSync } from 'svelte';
import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import type { Hub, HubResult } from '$lib/hub/hub';
import { installAnimations } from '../../../forms/animations.fixture';
import { button, input, leave, SAVED, save, settle, show, type } from '../form/form.fixture';
import { LOAD_SENTENCE, SAVE_SENTENCE } from './draft.svelte';
import { LIMIT_SENTENCE } from './policy';

const mocks = vi.hoisted(() => ({ hub: undefined as unknown }));
vi.mock('$lib/hub/context', () => ({ useHub: () => mocks.hub }));
vi.mock('motion', async (importOriginal) => ({
  ...(await importOriginal<typeof import('motion')>()),
  animate: (await import('$lib/motion/engine.fixture')).fakeRun,
}));

beforeEach(installAnimations);
afterEach(leave);

const POLICY = { sessionLimit: 5, recordSessions: false };
const ok = <T,>(value: T): HubResult<T> => ({ ok: true, value });
const down: HubResult<never> = { ok: false, error: { code: 'UNAVAILABLE', message: 'secret hub text', fields: [] } };

interface Sent {
  readonly operation: string;
  readonly variables: unknown;
}

/** A hub that answers the two policy operations in turn from `loads` and `saves`, and records every request. */
function hubWith(loads: HubResult<unknown>[], saves: HubResult<unknown>[] = []): Sent[] {
  const sent: Sent[] = [];
  mocks.hub = {
    request: (document: TypedDocumentNode, variables: unknown) => {
      const operation = /(?:mutation|query) (\w+)/.exec(print(document))?.[1] ?? '';
      sent.push({ operation, variables });
      return Promise.resolve((operation === 'DevicePolicy' ? loads : saves).shift() ?? down);
    },
  } as unknown as Hub;
  return sent;
}

function open(): void {
  document.querySelector<HTMLButtonElement>('[data-section="sessions"] h3 button')?.click();
  flushSync();
}

const limit = () => input('policy-sessionLimit');
const record = () => document.querySelector<HTMLInputElement>('input[type="checkbox"][aria-describedby="device-policy-record-hint"]');
const message = () => document.getElementById('device-policy-message')?.textContent ?? '';

async function editing(loads: HubResult<unknown>[], saves: HubResult<unknown>[] = []) {
  const sent = hubWith(loads, saves);
  const calls = show(SAVED);
  await settle();
  open();
  return { sent, calls };
}

it('RA-036, RA-040: a saved device reads the limit and recording through their own document', async () => {
  const { sent } = await editing([ok({ device: { ...POLICY, recordSessions: true } })]);
  expect(sent).toEqual([{ operation: 'DevicePolicy', variables: { id: 'd1' } }]);
  expect(limit().value).toBe('5');
  expect(record()?.checked).toBe(true);
});

it('RA-036: the help text says viewers and file tunnels do not count; RA-040: recordings are encrypted and badged', async () => {
  await editing([ok({ device: POLICY })]);
  expect(document.body.textContent).toContain('Viewers and file tunnels do not count');
  expect(document.body.textContent).toContain('encrypted at rest');
  expect(document.body.textContent).toContain('"Recording" badge');
});

it('RA-036, RA-040: the policy saves through its own mutation, and the device update stays out of it', async () => {
  const { sent, calls } = await editing([ok({ device: POLICY })], [ok({ updateDevice: { sessionLimit: 7, recordSessions: true } })]);
  type('policy-sessionLimit', '7');
  record()?.click();
  flushSync();
  await save();
  expect(sent[1]).toEqual({ operation: 'SetDevicePolicy', variables: { id: 'd1', input: { sessionLimit: 7, recordSessions: true } } });
  expect(calls.onSave).toHaveBeenCalledTimes(1);
  expect(calls.onSave.mock.calls[0]?.[0]).not.toHaveProperty('policy');
});

it('RA-036: emptying a stored limit sends 0', async () => {
  const { sent } = await editing([ok({ device: POLICY })], [ok({ updateDevice: { sessionLimit: null, recordSessions: false } })]);
  type('policy-sessionLimit', '');
  await save();
  expect(sent[1]?.variables).toEqual({ id: 'd1', input: { sessionLimit: 0 } });
});

it('RA-036: an untouched policy sends nothing', async () => {
  const { sent, calls } = await editing([ok({ device: POLICY })]);
  type('name', 'nas2');
  await save();
  expect(sent.map((request) => request.operation)).toEqual(['DevicePolicy']);
  expect(calls.onSave).toHaveBeenCalledTimes(1);
});

it('RA-036: 101 and 2.5 are refused with a fixed sentence, and nothing is sent', async () => {
  const { sent, calls } = await editing([ok({ device: POLICY })]);
  for (const text of ['101', '2.5']) {
    type('policy-sessionLimit', text);
    await save();
    expect(message()).toBe(LIMIT_SENTENCE);
  }
  expect(calls.onSave).not.toHaveBeenCalled();
  expect(sent).toHaveLength(1);
});

it('RA-036: 0 on create is refused', async () => {
  const calls = show(null, { sites: [{ id: 's1', name: 'Home', subnet: '10.0.0.0/24' }] });
  type('name', 'new');
  type('host', 'new.home.arpa');
  open();
  type('policy-sessionLimit', '0');
  await save();
  expect(message()).toBe(LIMIT_SENTENCE);
  expect(calls.onSave).not.toHaveBeenCalled();
});

it('RA-040: recording is off by default for a new device, and a ticked one goes to create', async () => {
  const calls = show(null, { sites: [{ id: 's1', name: 'Home', subnet: '10.0.0.0/24' }] });
  open();
  expect(record()?.checked).toBe(false);
  expect(limit().disabled).toBe(false);
  type('name', 'new');
  type('host', 'new.home.arpa');
  record()?.click();
  type('policy-sessionLimit', '4');
  await save();
  expect(calls.onSave.mock.calls[0]?.[0].policy).toEqual({ sessionLimit: '4', recordSessions: true });
});

it('RA-036: a failed policy load keeps the rest of the form usable, sends no policy, and offers Try again', async () => {
  const { sent, calls } = await editing([down, ok({ device: POLICY })]);
  expect(message()).toBe(LOAD_SENTENCE);
  expect(message()).not.toContain('secret');
  expect(limit().disabled).toBe(true);
  expect(record()?.disabled).toBe(true);
  type('name', 'nas2');
  await save();
  expect(calls.onSave).toHaveBeenCalledTimes(1);
  expect(sent.map((request) => request.operation)).toEqual(['DevicePolicy']);
  button('Try again').click();
  await settle();
  expect(limit().disabled).toBe(false);
  expect(limit().value).toBe('5');
  expect(message()).toBe('');
});

it('RA-036: the policy fields keep their geometry while loading: the hint and message lines are always there', async () => {
  hubWith([new Promise<never>(() => undefined) as never]);
  show(SAVED);
  open();
  expect(limit().disabled).toBe(true);
  expect(document.getElementById('device-policy-sessionLimit-hint')).not.toBeNull();
  expect(document.getElementById('device-policy-message')).not.toBeNull();
});

it('RA-036: a policy save the hub refuses says so, and the device is not saved', async () => {
  const { calls } = await editing([ok({ device: POLICY })], [down]);
  type('policy-sessionLimit', '9');
  await save();
  expect(message()).toBe(SAVE_SENTENCE);
  expect(calls.onSave).not.toHaveBeenCalled();
});
