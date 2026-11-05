import type { DocumentNode, OperationDefinitionNode } from 'graphql';
import type { Hub, HubResult } from '$lib/hub/hub';
import { fakeHub } from '../hub.fixture';
import { devicesAnswer } from './screen.fixture.svelte';
import type { CardDevice } from '../api';

/** One `Devices` request that has not been answered yet. */
export interface Held {
  readonly variables: Record<string, unknown>;
  /** Answer it with `devices`; `next` is the end cursor of a page that has another one. */
  release(devices: readonly CardDevice[], next?: string | null): void;
}

function nameOf(document: DocumentNode): string {
  const operation = document.definitions.find(
    (definition): definition is OperationDefinitionNode => definition.kind === 'OperationDefinition',
  );
  return operation?.name?.value ?? '';
}

/** A fake hub whose `Devices` requests wait in `held` until a test releases them, in any order. */
export function heldHub() {
  const fake = fakeHub({ Tags: { tags: [] } });
  const held: Held[] = [];
  const hub: Hub = {
    ...fake.hub,
    request: ((document: DocumentNode, variables: Record<string, unknown>) => {
      if (nameOf(document) !== 'Devices') return fake.hub.request(document as never, variables as never);
      return new Promise<HubResult<unknown>>((resolve) => {
        held.push({
          variables,
          release: (devices, next = null) => {
            resolve({ ok: true, value: devicesAnswer(devices, next) });
          },
        });
      });
    }) as Hub['request'],
  };
  return { ...fake, hub, held };
}
