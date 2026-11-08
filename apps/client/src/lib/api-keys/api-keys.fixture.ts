// Test support: a hub that answers the three API key operations from canned results and records what it was sent,
// and the synthetic values the tests use. Never a real key (WP-50 plan checkpoint 4).
import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { print } from 'graphql';
import { flushSync } from 'svelte';
import { apiKeys } from '$lib/fixtures/phase3';
import type { Hub, HubError, HubResult } from '$lib/hub/hub';
import type { ApiKey } from './api';

/** A made-up key in the K1 format: `anima_` and 43 base64url characters. */
export const FAKE_SECRET = `anima_${'A'.repeat(43)}`;

/** The tests' clock: after the K6 key "old-script" expired, before "metrics-scrape" does. */
export const NOW = new Date('2026-11-07T12:00:00Z');

/** The K6 `apiKeys` slice. */
export const FIXTURE_KEYS: readonly ApiKey[] = apiKeys;

export const MADE_KEY: ApiKey = {
  id: '0190a000-0000-7000-8019-0000000000aa',
  name: 'backup-job',
  scope: 'WRITE',
  prefix: 'anima_AAAAAA',
  expiresAt: '2027-02-05T12:00:00Z',
  createdAt: '2026-11-07T12:00:00Z',
  lastUsedAt: null,
};

export type Operation = 'ApiKeys' | 'CreateApiKey' | 'RevokeApiKey';

export interface SentRequest {
  readonly operation: string;
  readonly variables: unknown;
}

export function ok<T>(value: T): HubResult<T> {
  return { ok: true, value };
}

export function failure(code: HubError['code'], message = 'hub text'): HubResult<never> {
  return { ok: false, error: { code, message, fields: [] } as HubError };
}

/** `answers` maps an operation to its result; an operation without one is `unreachable`. */
export function fakeHub(answers: Partial<Record<Operation, HubResult<unknown> | Promise<HubResult<unknown>>>>): { hub: Hub; sent: SentRequest[] } {
  const sent: SentRequest[] = [];
  const hub = {
    request: (document: TypedDocumentNode, variables: unknown) => {
      const operation = /(?:mutation|query) (\w+)/.exec(print(document))?.[1] ?? '';
      sent.push({ operation, variables });
      return Promise.resolve(answers[operation as Operation] ?? failure('unreachable'));
    },
  } as unknown as Hub;
  return { hub, sent };
}

export async function settled(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}

export function button(label: string): HTMLButtonElement {
  const found = [...document.querySelectorAll('button')].find((candidate) => candidate.textContent.trim() === label);
  if (found === undefined) throw new Error(`no button ${label}`);
  return found;
}

/** Types into a text or date field the way the browser reports it. */
export function typeInto(field: HTMLInputElement, value: string): void {
  field.value = value;
  field.dispatchEvent(new Event('input', { bubbles: true }));
  flushSync();
}

/** The radio button whose label reads `label`. */
export function radio(label: string): HTMLInputElement {
  const found = [...document.querySelectorAll('label')].find((candidate) => candidate.textContent.trim().startsWith(label));
  const input = found?.querySelector('input[type="radio"]');
  if (!(input instanceof HTMLInputElement)) throw new Error(`no radio ${label}`);
  return input;
}

export function field(selector: string): HTMLInputElement {
  const found = document.querySelector(selector);
  if (!(found instanceof HTMLInputElement)) throw new Error(`no field ${selector}`);
  return found;
}
