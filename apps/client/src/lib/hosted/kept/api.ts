import { graphql } from '$lib/hub/generated';
import type { HubKeptCountsQuery } from '$lib/hub/generated/graphql';
import type { Hub, HubResult } from '$lib/hub/hub';
import { retentionApi } from '$lib/retention/api';
import type { HubKept } from './kept';

// K1 has no count of checks: the page counts the checks of each device, 200 devices a page.
const HubKeptCounts = graphql(`
  query HubKeptCounts($after: String) {
    devices(first: 200, after: $after) {
      totalCount
      pageInfo {
        hasNextPage
        endCursor
      }
      edges {
        node {
          checks {
            id
          }
        }
      }
    }
  }
`);

/** At most this many pages are read: the hub's page limit times a bound no home lab reaches. */
const MAX_PAGES = 50;

type Counts = Pick<HubKept, 'devices' | 'checks'>;

/** The count of devices, and of checks only when every page was read: a part of the checks is not their count. */
async function readCounts(hub: Hub): Promise<Counts> {
  let devices: number | null = null;
  let checks = 0;
  let after: string | null = null;
  for (let page = 0; page < MAX_PAGES; page += 1) {
    const result: HubResult<HubKeptCountsQuery> = await hub.request(HubKeptCounts, { after });
    if (!result.ok) return { devices, checks: null };
    const { totalCount, edges, pageInfo }: HubKeptCountsQuery['devices'] = result.value.devices;
    devices = totalCount;
    checks += edges.reduce((sum, edge) => sum + edge.node.checks.length, 0);
    if (!pageInfo.hasNextPage) return { devices, checks };
    after = pageInfo.endCursor ?? null;
    if (after === null) break;
  }
  return { devices, checks: null };
}

/** What the hub keeps, from the hub: the counts of its devices and of its checks, and its retention periods (HMD-073). */
export async function readKept(hub: Hub): Promise<HubKept> {
  const [counts, periods] = await Promise.all([readCounts(hub), retentionApi(hub).list()]);
  return { ...counts, periods: periods.ok ? periods.value : null };
}
