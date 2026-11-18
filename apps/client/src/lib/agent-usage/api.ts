import { graphql } from '$lib/hub/generated';
import type { AgentUsageFilter, AgentUsageQuery } from '$lib/hub/generated/graphql';
import type { AgentUsageDimension } from '$lib/hub/generated/graphql';
import type { Hub, HubResult } from '$lib/hub/hub';

// `everyone` asks for one day of the use with no device or program filter. Its device list stays whole when the operator
// filters by one device, so the device choice still has every device whose daemon sends counts.
const AGENT_USAGE = graphql(`
  query AgentUsage($filter: AgentUsageFilter, $by: [AgentUsageDimension!]!, $everyone: AgentUsageFilter) {
    agentUsage(filter: $filter, by: $by) {
      firstDay
      lastDay
      rowsLeftOut
      pricesAsOf
      rows {
        ...AgentUsageSums
        device {
          id
          name
        }
        source
        model
        repository
        day
      }
      total {
        ...AgentUsageSums
      }
      devices {
        state
        device {
          id
          name
        }
      }
    }
    everyone: agentUsage(filter: $everyone, by: []) {
      devices {
        state
        device {
          id
          name
        }
      }
    }
  }
`);

// The fragment is only read by codegen, which inlines it into the document that spreads it.
void graphql(`
  fragment AgentUsageSums on AgentUsageRow {
    sessions
    prompts
    toolCalls
    inputTokens
    outputTokens
    cacheReadTokens
    cacheWriteTokens
    estimatedCostUsd
    unpricedTokens
  }
`);

export type UsageAnswer = AgentUsageQuery['agentUsage'];
export type UsageRow = UsageAnswer['rows'][number];
export type UsageSums = UsageAnswer['total'];
export type UsageDevice = UsageAnswer['devices'][number];

/** The use of the AI agent programs, as the hub sums it, and every device that reports it. */
export interface AgentUsageView {
  readonly usage: UsageAnswer;
  readonly everyDevice: readonly UsageDevice[];
}

/** What one read asks for. */
export interface UsageRequest {
  readonly filter: AgentUsageFilter;
  readonly by: readonly AgentUsageDimension[];
}

/** The agent usage call of the hub (AIS-020). */
export function agentUsageApi(hub: Pick<Hub, 'request'>) {
  return {
    read: async ({ filter, by }: UsageRequest): Promise<HubResult<AgentUsageView>> => {
      const day = filter.lastDay ?? undefined;
      const everyone: AgentUsageFilter = day === undefined ? {} : { firstDay: day, lastDay: day };
      const result = await hub.request(AGENT_USAGE, { filter, by: [...by], everyone });
      if (!result.ok) return result;
      return { ok: true, value: { usage: result.value.agentUsage, everyDevice: result.value.everyone.devices } };
    },
  };
}
