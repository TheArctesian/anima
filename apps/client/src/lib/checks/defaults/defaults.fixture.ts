// Test support: a hub that answers the two default-check-settings operations from canned results and records
// what it was sent, and synthetic values that differ from the K1 input defaults (60, 3, 20, 1000).
import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { print } from 'graphql';
import { flushSync } from 'svelte';
import type { CheckDefaults } from '$lib/checks/api';
import type { Hub, HubError, HubResult } from '$lib/hub/hub';

export const STORED: CheckDefaults = { intervalSeconds: 120, retries: 5, retryIntervalSeconds: 45, degradedThresholdMs: 2500 };

export type Operation = 'CheckDefaults' | 'SetCheckDefaults';

export interface SentRequest {
  readonly operation: string;
  readonly variables: unknown;
}

export function ok<T>(value: T): HubResult<T> {
  return { ok: true, value };
}

export function failure(code: HubError['code'], message = 'secret hub text'): HubResult<never> {
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
