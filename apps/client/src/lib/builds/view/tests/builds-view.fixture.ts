// Test support: a hub that answers the Builds view's operations from canned results, records what it was sent, and lets a
// test push subscription changes.
import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { print } from 'graphql';
import { flushSync } from 'svelte';
import type { Hub, HubError, HubResult } from '$lib/hub/hub';
import type { JobChange, JobView, ReleaseView } from '../api';

export const COMMIT = '0123456789abcdef0123456789abcdef01234567';

export function job(overrides: Partial<JobView> = {}): JobView {
  return {
    id: 'job-1',
    releaseId: 'rel-1',
    target: 'LINUX_X86_64',
    state: 'QUEUED',
    failureReason: null,
    attempts: 0,
    builderName: null,
    stateChangedAt: '2026-11-12T11:00:00Z',
    ...overrides,
  };
}

export function release(id: string, version: string, createdAt: string, jobs: JobView[]): ReleaseView {
  return { id, version, commit: COMMIT, createdAt, createdBy: 'operator', jobs };
}

export function ok<T>(value: T): HubResult<T> {
  return { ok: true, value };
}

export function refused(message: string): HubResult<never> {
  const error: HubError = { code: 'VALIDATION', message: 'Invalid input', fields: [{ field: 'version', message }] };
  return { ok: false, error };
}

export type Answers = Record<string, HubResult<unknown> | (() => HubResult<unknown>)>;

export interface SentRequest {
  readonly operation: string;
  readonly variables: Record<string, unknown>;
}

/** `answers` maps an operation to its result; one with no answer is `unreachable`. */
export function fakeHub(answers: Answers) {
  const sent: SentRequest[] = [];
  let push: (change: JobChange) => void = () => undefined;
  const hub = {
    request: (document: TypedDocumentNode, variables: Record<string, unknown>) => {
      const operation = /(?:mutation|query) (\w+)/.exec(print(document))?.[1] ?? '';
      sent.push({ operation, variables });
      const answer = answers[operation];
      const result = typeof answer === 'function' ? answer() : answer;
      return Promise.resolve(result ?? { ok: false, error: { code: 'unreachable', message: 'The hub did not answer.' } });
    },
    subscribe: (_document: TypedDocumentNode, _variables: unknown, sink: { next: (value: unknown) => void }) => {
      push = (change) => {
        sink.next({ buildJobUpdated: change });
      };
      return () => undefined;
    },
  } as unknown as Hub;
  return { hub, sent, push: (change: JobChange) => { push(change); } };
}

export async function settled(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  flushSync();
}
