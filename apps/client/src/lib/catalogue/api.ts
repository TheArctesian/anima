import { graphql } from '$lib/hub/generated';
import type { CatalogueEntriesQuery } from '$lib/hub/generated/graphql';
import type { Hub, HubResult } from '$lib/hub/hub';

const CATALOGUE_ENTRIES = graphql(`
  query CatalogueEntries {
    catalogueEntries {
      id
      name
      description
      kind
      version
      author
      tags
      documentationUrl
      marks
      networkPorts {
        port
        protocols
      }
      lastMachineTest
      pinned
      install {
        refusal
        reason
        needsOpenToNetwork
      }
      environmentVariables
      composeFile
    }
  }
`);

export type CatalogueEntry = CatalogueEntriesQuery['catalogueEntries'][number];

/** What the catalogue screen asks of the hub (task H38); the screen and its tests take this, not the hub. */
export interface CatalogueCalls {
  /** The entries that the hub lists: each recipe that passed the static test. The hub reaches no machine. */
  entries(): Promise<HubResult<readonly CatalogueEntry[]>>;
}

/** The catalogue calls of the hub. */
export function catalogueApi(hub: Pick<Hub, 'request'>): CatalogueCalls {
  return {
    entries: async () => {
      const result = await hub.request(CATALOGUE_ENTRIES, {});
      return result.ok ? { ok: true, value: result.value.catalogueEntries } : result;
    },
  };
}
