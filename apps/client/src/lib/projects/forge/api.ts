import { graphql } from '$lib/hub/generated';
import type { HubResult } from '$lib/hub/hub';
import { codeApi } from '../code/api';
import { historyApi } from '../history/api';
import { workApi } from '../work/api';
import { field, type Asker } from './request';
import type { RepositoryAccess } from './types';

// The fragments every tab's lists share; only codegen reads them, inlining them where they are spread (API-008).
void graphql(`
  fragment ForgeReadFields on ForgeRead {
    at
    stale
    problem
  }
`);

void graphql(`
  fragment ForgePageFields on PageInfo {
    hasNextPage
    endCursor
  }
`);

void graphql(`
  fragment CommitChecksFields on CommitChecks {
    state
    passed
    failed
    pending
    failing
  }
`);

const PROJECT_ACCESS = graphql(`
  query ProjectAccess($slug: String!) {
    project(slug: $slug) {
      repositories {
        id
        access
      }
    }
  }
`);

/** Whether the hub can read each of the project's repositories, by repository id (PRJ-031). */
async function access(hub: Asker, slug: string): Promise<HubResult<ReadonlyMap<string, RepositoryAccess>>> {
  const result = await field(hub, PROJECT_ACCESS, 'project', { slug });
  if (!result.ok) return result;
  return { ok: true, value: new Map((result.value?.repositories ?? []).map((each) => [each.id, each.access])) };
}

/** The hub's calls for a project's work, code and history (PRJ-027 to PRJ-035). */
export function forgeApi(hub: Asker) {
  return { access: (slug: string) => access(hub, slug), ...workApi(hub), ...codeApi(hub), ...historyApi(hub) };
}

export type ForgeApi = ReturnType<typeof forgeApi>;
export type { RunChange } from '../work/api';
