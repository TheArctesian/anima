// Test support: a hub that answers the three access operations from canned results and records what it was sent.
import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { print } from 'graphql';
import type { Hub, HubError, HubResult } from '$lib/hub/hub';
import { parseHubAddress } from '$lib/hub/hub';

const parsed = parseHubAddress('hub.example.org');
if (!parsed.ok) throw new Error('bad fixture address');
/** Not the default hub, so a test fails when code uses the default instead of `hub.address` (CLI-003). */
export const HUB_ADDRESS = parsed.value;

export type Operation = 'Login' | 'Logout' | 'Me';

export interface SentRequest {
  readonly operation: string;
  readonly variables: unknown;
}

export function failure(code: 'UNAUTHENTICATED' | 'unreachable' | 'INTERNAL', message = 'hub text'): HubResult<never> {
  return { ok: false, error: { code, message } as HubError };
}

export function ok<T>(value: T): HubResult<T> {
  return { ok: true, value };
}

/** `answers` maps an operation name to the data it returns; an operation without an answer is `unreachable`. */
export function fakeHub(answers: Partial<Record<Operation, HubResult<unknown>>>): { hub: Hub; sent: SentRequest[] } {
  const sent: SentRequest[] = [];
  const hub = {
    address: HUB_ADDRESS,
    request: (document: TypedDocumentNode, variables: unknown) => {
      const operation = /(?:mutation|query) (\w+)/.exec(print(document))?.[1] ?? '';
      sent.push({ operation, variables });
      return Promise.resolve(answers[operation as Operation] ?? failure('unreachable'));
    },
  } as unknown as Hub;
  return { hub, sent };
}
