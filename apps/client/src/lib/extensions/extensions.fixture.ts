import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import type { Extension } from './api';
import type { Hub, HubError, HubResult } from '$lib/hub/hub';

// Test support: extensions in the hub's shape and a hub that keeps their switches in memory.

function entry(key: string, name: string, more: Partial<Extension> = {}): Extension {
  return { key, name, built: true, fixed: false, on: false, needs: [], worksWith: [], neededBy: [], ...more };
}

export const FIXTURE_EXTENSIONS: readonly Extension[] = [
  entry('map', 'Map', { built: false }),
  entry('projects', 'Projects'),
  entry('secrets', 'Secrets'),
  entry('integrations', 'Integrations', { fixed: true, on: true }),
  entry('model_gateway', 'Model gateway', { needs: ['secrets'] }),
];

export interface FakeHub extends Pick<Hub, 'request'> {
  /** The switches sent, as `key:on`, in order. */
  readonly sent: string[];
  /** What the next `setExtension` answers instead of changing anything. */
  refuse: HubError | null;
}

/** A hub that answers `Extensions` from `start` and applies `SetExtension` as the hub does (EXT-011). */
export function fakeHub(start: readonly Extension[] = FIXTURE_EXTENSIONS): FakeHub {
  let all = start;
  const hub: FakeHub = {
    sent: [],
    refuse: null,
    request: <TResult, TVariables extends Record<string, unknown>>(document: TypedDocumentNode<TResult, TVariables>, variables: TVariables) => {
      const name = (document.definitions[0] as { name?: { value: string } }).name?.value;
      if (name === 'Extensions') return Promise.resolve({ ok: true, value: { extensions: all } } as unknown as HubResult<TResult>);
      const key = String(variables['key']);
      const on = variables['on'] === true;
      hub.sent.push(`${key}:${String(on)}`);
      if (hub.refuse !== null) return Promise.resolve({ ok: false, error: hub.refuse } as HubResult<TResult>);
      const keys = new Set(on ? [key, ...(all.find((each) => each.key === key)?.needs ?? [])] : [key]);
      all = all.map((each) => (keys.has(each.key) ? { ...each, on } : each));
      all = all.map((each) => ({ ...each, neededBy: all.filter((other) => other.on && other.needs.includes(each.key)).map((other) => other.key) }));
      return Promise.resolve({ ok: true, value: { setExtension: all.filter((each) => keys.has(each.key)) } } as unknown as HubResult<TResult>);
    },
  };
  return hub;
}
