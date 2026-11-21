import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import type { Hub, HubError, HubResult } from '$lib/hub/hub';
import type { DayRow, DeviceRow, ModelRow, RepositoryRow, UsageDevice, UsageSums } from './api';

// Test support: an answer in the hub's shape and a hub that gives it, recording what the page asked for.

const MAC = { id: 'd-mac', name: 'mac-mini', batteries: [] };
const LINUX = { id: 'd-nas', name: 'nas', batteries: [] };
const BOX = { id: 'd-box', name: 'gpu-box', batteries: [] };

export const DEVICES: readonly UsageDevice[] = [
  { device: BOX, state: 'TURNED_OFF' },
  { device: MAC, state: 'READING' },
  { device: LINUX, state: 'SYSTEM_UNIT' },
];

const NONE: UsageSums = { prompts: 0, toolCalls: 0, inputTokens: 0, outputTokens: 0, cacheReadTokens: 0, cacheWriteTokens: 0, estimatedCostUsd: 0, unpricedTokens: 0, sessions: 0 };

/** The counts of a group; what `change` leaves out is none. */
export function sums(change: Partial<UsageSums> = {}): UsageSums {
  return { ...NONE, ...change };
}

const OPUS = { source: 'CLAUDE_CODE', model: 'claude-opus-4' } as const;
const CODEX = { source: 'CODEX', model: 'gpt-5-codex' } as const;

/** What the hub answers for the choices of the page; the days are 2026-11-11 to 2026-11-17. */
export interface HubUsage {
  firstDay: string;
  lastDay: string;
  pricesAsOf: string;
  total: UsageSums;
  days: readonly DayRow[];
  models: readonly ModelRow[];
  devices: readonly DeviceRow[];
  states: readonly UsageDevice[];
  repositories: readonly RepositoryRow[];
  everything: UsageSums;
  newest: readonly { repository: string | null; day: string }[];
  repositoriesLeftOut: number;
}

export const ANSWER: HubUsage = {
  firstDay: '2026-11-11',
  lastDay: '2026-11-17',
  pricesAsOf: '2026-11-01',
  total: sums({ sessions: 17, prompts: 169, toolCalls: 1330, inputTokens: 19_250, outputTokens: 424_000, cacheReadTokens: 2_200_000, cacheWriteTokens: 61_000, estimatedCostUsd: 12.4, unpricedTokens: 800 }),
  days: [
    { ...sums({ sessions: 5, estimatedCostUsd: 4, inputTokens: 1000 }), ...OPUS, day: '2026-11-17' },
    { ...sums({ sessions: 2, estimatedCostUsd: 1, inputTokens: 500 }), ...CODEX, day: '2026-11-17' },
    { ...sums({ sessions: 8, estimatedCostUsd: 10, inputTokens: 4000 }), ...OPUS, day: '2026-11-13' },
  ],
  models: [
    { ...sums({ sessions: 12, prompts: 140, toolCalls: 1210, inputTokens: 18_300, outputTokens: 420_000, cacheReadTokens: 2_200_000, cacheWriteTokens: 61_000, estimatedCostUsd: 9.3 }), ...OPUS },
    { ...sums({ sessions: 5, prompts: 29, toolCalls: 120, inputTokens: 950, outputTokens: 4000, estimatedCostUsd: 3.1 }), ...CODEX },
  ],
  devices: [{ ...sums({ sessions: 17, prompts: 169, inputTokens: 19_250, estimatedCostUsd: 12.4 }), device: MAC }],
  states: [...DEVICES],
  repositories: [
    { ...sums({ sessions: 3, estimatedCostUsd: 0.5 }), repository: null },
    { ...sums({ sessions: 4, estimatedCostUsd: 7 }), repository: 'github.com/acme/blog' },
    { ...sums({ sessions: 10, estimatedCostUsd: 4.9 }), repository: 'github.com/acme/anima' },
  ],
  everything: sums({ sessions: 17, estimatedCostUsd: 12.4 }),
  newest: [
    { repository: 'github.com/acme/anima', day: '2026-11-17' },
    { repository: 'github.com/acme/anima', day: '2026-11-13' },
    { repository: 'github.com/acme/blog', day: '2026-11-14' },
    { repository: null, day: '2026-11-17' },
  ],
  repositoriesLeftOut: 0,
};

export interface FakeUsageHub extends Pick<Hub, 'request' | 'subscribe'> {
  /** The variables of each read, in order. */
  readonly asked: Record<string, unknown>[];
  /** How many subscriptions are open: the page listens for the charge of each battery (MET-032). */
  listening: number;
  /** What the next read answers instead of the answer. */
  fail: HubError | null;
  answer: HubUsage;
  everyone: readonly UsageDevice[];
}

/** A hub that answers `AgentUsage` with `answer`, and `everyone` as the devices of the whole fleet. */
export function fakeUsageHub(answer: HubUsage = ANSWER, everyone: readonly UsageDevice[] = answer.states): FakeUsageHub {
  const hub: FakeUsageHub = {
    asked: [],
    listening: 0,
    fail: null,
    answer,
    everyone,
    request: <TResult, TVariables extends Record<string, unknown>>(_document: TypedDocumentNode<TResult, TVariables>, variables: TVariables) => {
      hub.asked.push(variables);
      if (hub.fail !== null) return Promise.resolve({ ok: false, error: hub.fail } as HubResult<TResult>);
      const { answer: now } = hub;
      const value = {
        days: { firstDay: now.firstDay, lastDay: now.lastDay, pricesAsOf: now.pricesAsOf, rows: now.days, total: now.total },
        models: { rows: now.models },
        devices: { rows: now.devices, devices: now.states },
        repositories: { rowsLeftOut: now.repositoriesLeftOut, rows: now.repositories, total: now.everything },
        newest: { rows: now.newest },
        everyone: { devices: hub.everyone },
      };
      return Promise.resolve({ ok: true, value } as unknown as HubResult<TResult>);
    },
    subscribe: () => {
      hub.listening += 1;
      return () => {
        hub.listening -= 1;
      };
    },
  };
  return hub;
}
