// Test support: a hub that answers the two retention operations from canned results and records what it was sent.
import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { print } from 'graphql';
import { flushSync } from 'svelte';
import { retention } from '$lib/fixtures/phase3';
import type { Hub, HubError, HubResult } from '$lib/hub/hub';
import type { RetentionPeriod } from './api';

/** The K6 `retention` slice, the D-25 defaults. */
export const FIXTURE_PERIODS: readonly RetentionPeriod[] = retention;

export type Operation = 'Retention' | 'SetRetention';

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
