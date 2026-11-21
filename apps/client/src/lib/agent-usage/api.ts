import { graphql } from '$lib/hub/generated';
import type { AgentSource, AgentUsageFilter, AgentUsageState } from '$lib/hub/generated/graphql';
import type { Hub, HubResult } from '$lib/hub/hub';

// One read gives the page everything it draws (AIS-020). `$filter` holds the choices of the page; `$everything` is the
// same without the repository, so the list of repositories stays whole when one is chosen; `$everyone` asks one day with
// no device or program, so the device choice still has every device whose daemon sends counts.
const AGENT_USAGE = graphql(`
  query AgentUsage($filter: AgentUsageFilter, $everything: AgentUsageFilter, $everyone: AgentUsageFilter) {
    days: agentUsage(filter: $filter, by: [DAY, SOURCE, MODEL]) {
      firstDay
      lastDay
      pricesAsOf
      rows {
        ...AgentUsageSums
        source
        model
        day
      }
      total {
        ...AgentUsageSums
      }
    }
    models: agentUsage(filter: $filter, by: [SOURCE, MODEL]) {
      rows {
        ...AgentUsageSums
        source
        model
      }
    }
    devices: agentUsage(filter: $filter, by: [DEVICE]) {
      rows {
        ...AgentUsageSums
        device {
          id
          name
        }
      }
      devices {
        state
        device {
          id
          name
        }
      }
    }
    repositories: agentUsage(filter: $everything, by: [REPOSITORY]) {
      rowsLeftOut
      rows {
        ...AgentUsageSums
        repository
      }
      total {
        ...AgentUsageSums
      }
    }
    newest: agentUsage(filter: $everything, by: [REPOSITORY, DAY]) {
      rows {
        repository
        day
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
    sessions: agentSessions(filter: $filter, first: 50) {
      sessionsLeftOut
      sessions {
        id
        device {
          id
          name
        }
        source
        model
        repository
        startedAt
        lastUseAt
        recent
        prompts
        toolCalls
        inputTokens
        outputTokens
        cacheReadTokens
        cacheWriteTokens
        estimatedCostUsd
        unpricedTokens
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

/** The counts of one group. The cost is null when no model of the group has a known price. */
export interface UsageSums {
  readonly sessions: number;
  readonly prompts: number;
  readonly toolCalls: number;
  readonly inputTokens: number;
  readonly outputTokens: number;
  readonly cacheReadTokens: number;
  readonly cacheWriteTokens: number;
  readonly estimatedCostUsd: number | null;
  readonly unpricedTokens: number;
}

export interface NamedDevice {
  readonly id: string;
  readonly name: string;
}

/** A device whose daemon sent a usage state (AIS-024). */
export interface UsageDevice {
  readonly device: NamedDevice;
  readonly state: AgentUsageState;
}

export type DayRow = UsageSums & { readonly day: string; readonly source: AgentSource | null; readonly model: string | null };
export type ModelRow = UsageSums & { readonly source: AgentSource | null; readonly model: string | null };
export type DeviceRow = UsageSums & { readonly device: NamedDevice };
/** `repository` is null for the use that has none. */
export type RepositoryRow = UsageSums & { readonly repository: string | null };

/** One session with its sums over the days of the page (AIS-025). `recent`: used in the last 30 minutes (AIS-026). */
export type SessionRow = Omit<UsageSums, 'sessions'> & {
  readonly id: string;
  readonly device: NamedDevice;
  readonly source: AgentSource;
  readonly model: string | null;
  readonly repository: string | null;
  readonly startedAt: string;
  readonly lastUseAt: string;
  readonly recent: boolean;
};

/** The use of the AI agent programs for the choices of the page, as the hub sums it. */
export interface AgentUsageView {
  readonly firstDay: string;
  readonly lastDay: string;
  /** The day of the public list prices of the estimate. */
  readonly pricesAsOf: string;
  /** Everything that the choices admit. */
  readonly total: UsageSums;
  readonly days: readonly DayRow[];
  readonly models: readonly ModelRow[];
  readonly devices: readonly DeviceRow[];
  /** The state of each device that the choices admit. */
  readonly states: readonly UsageDevice[];
  /** Each repository with use, whatever repository is chosen. */
  readonly repositories: readonly RepositoryRow[];
  /** The total with no repository chosen: the line "All repositories". */
  readonly everything: UsageSums;
  /** The newest day with use of each repository. */
  readonly newest: readonly { readonly repository: string | null; readonly day: string }[];
  /** How many repositories are not in `repositories` because of the hub's limit of rows. */
  readonly repositoriesLeftOut: number;
  /** Every device that reports, whatever the choices. */
  readonly everyDevice: readonly UsageDevice[];
  /** The single sessions that the choices admit, the newest use first: at most 50. */
  readonly sessions: readonly SessionRow[];
  /** How many sessions are not in `sessions`. */
  readonly sessionsLeftOut: number;
}

// The two types below serve only `report/columns.ts` and `UsageNotes.svelte`, which the page no longer uses and which
// wait for a person to take them out of the repository.
export type UsageRow = UsageSums & {
  readonly device: NamedDevice | null;
  readonly source: AgentSource | null;
  readonly model: string | null;
  readonly repository: string | null;
  readonly day: string | null;
};
export interface UsageAnswer {
  readonly pricesAsOf: string;
  readonly rowsLeftOut: number;
  readonly total: UsageSums;
  readonly devices: readonly UsageDevice[];
}

/** What one read asks for. */
export interface UsageRequest {
  readonly filter: AgentUsageFilter;
}

/** `filter` with no repository. */
function everyRepository(filter: AgentUsageFilter): AgentUsageFilter {
  return Object.fromEntries(Object.entries(filter).filter(([key]) => key !== 'repository'));
}

/** The agent usage call of the hub (AIS-020). */
export function agentUsageApi(hub: Pick<Hub, 'request'>) {
  return {
    read: async ({ filter }: UsageRequest): Promise<HubResult<AgentUsageView>> => {
      const day = filter.lastDay ?? undefined;
      const everyone: AgentUsageFilter = day === undefined ? {} : { firstDay: day, lastDay: day };
      const result = await hub.request(AGENT_USAGE, { filter, everything: everyRepository(filter), everyone });
      if (!result.ok) return result;
      const { days, models, devices, repositories, newest, everyone: all, sessions } = result.value;
      const withDay = days.rows.flatMap((row) => (row.day == null ? [] : [{ ...row, day: row.day }]));
      const withDevice = devices.rows.flatMap((row) => (row.device == null ? [] : [{ ...row, device: row.device }]));
      const lastUse = newest.rows.flatMap((row) => (row.day == null ? [] : [{ repository: row.repository ?? null, day: row.day }]));
      return {
        ok: true,
        value: {
          firstDay: days.firstDay,
          lastDay: days.lastDay,
          pricesAsOf: days.pricesAsOf,
          total: days.total,
          days: withDay.map((row) => ({ ...row, source: row.source ?? null, model: row.model ?? null })),
          models: models.rows.map((row) => ({ ...row, source: row.source ?? null, model: row.model ?? null })),
          devices: withDevice,
          states: devices.devices,
          repositories: repositories.rows.map((row) => ({ ...row, repository: row.repository ?? null })),
          everything: repositories.total,
          newest: lastUse,
          repositoriesLeftOut: repositories.rowsLeftOut,
          everyDevice: all.devices,
          sessions: sessions.sessions.map((row) => ({ ...row, model: row.model ?? null, repository: row.repository ?? null })),
          sessionsLeftOut: sessions.sessionsLeftOut,
        },
      };
    },
  };
}
