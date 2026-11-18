import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import type { Hub, HubError, HubResult } from '$lib/hub/hub';
import type { UsageAnswer, UsageDevice, UsageRow } from './api';

// Test support: an answer in the hub's shape and a hub that gives it, recording what the page asked for.

const MAC = { id: 'd-mac', name: 'mac-mini' };
const LINUX = { id: 'd-nas', name: 'nas' };
const BOX = { id: 'd-box', name: 'gpu-box' };

export const DEVICES: readonly UsageDevice[] = [
  { device: BOX, state: 'TURNED_OFF' },
  { device: MAC, state: 'READING' },
  { device: LINUX, state: 'SYSTEM_UNIT' },
];

const NONE = { prompts: 0, toolCalls: 0, inputTokens: 0, outputTokens: 0, cacheReadTokens: 0, cacheWriteTokens: 0, estimatedCostUsd: 0, unpricedTokens: 0, sessions: 0 };

export function row(change: Partial<UsageRow>): UsageRow {
  return { ...NONE, device: null, source: null, model: null, repository: null, day: null, ...change };
}

export const ROWS: readonly UsageRow[] = [
  row({ device: MAC, source: 'CLAUDE_CODE', sessions: 12, prompts: 140, toolCalls: 1210, inputTokens: 18_300, outputTokens: 420_000, cacheReadTokens: 2_200_000, cacheWriteTokens: 61_000, estimatedCostUsd: 12.4 }),
  row({ device: MAC, source: 'CODEX', sessions: 3, prompts: 20, toolCalls: 90, inputTokens: 950, outputTokens: 4_000, estimatedCostUsd: 0.003, unpricedTokens: 0 }),
  row({ device: BOX, source: 'CODEX', sessions: 2, prompts: 9, toolCalls: 30, inputTokens: 1_400_000_000, outputTokens: 800, estimatedCostUsd: null, unpricedTokens: 1_400_000_800 }),
];

export const ANSWER: UsageAnswer = {
  firstDay: '2026-10-19',
  lastDay: '2026-11-17',
  rowsLeftOut: 0,
  pricesAsOf: '2026-11-01',
  rows: [...ROWS],
  total: row({ sessions: 17, prompts: 169, toolCalls: 1330, inputTokens: 1_400_019_250, outputTokens: 424_800, cacheReadTokens: 2_200_000, cacheWriteTokens: 61_000, estimatedCostUsd: 12.403, unpricedTokens: 1_400_000_800 }),
  devices: [...DEVICES],
};

export interface FakeUsageHub extends Pick<Hub, 'request'> {
  /** The variables of each read, in order. */
  readonly asked: Record<string, unknown>[];
  /** What the next read answers instead of the answer. */
  fail: HubError | null;
  answer: UsageAnswer;
  everyone: readonly UsageDevice[];
}

/** A hub that answers `AgentUsage` with `answer`, and `everyone` as the devices of the whole fleet. */
export function fakeUsageHub(answer: UsageAnswer = ANSWER, everyone: readonly UsageDevice[] = answer.devices): FakeUsageHub {
  const hub: FakeUsageHub = {
    asked: [],
    fail: null,
    answer,
    everyone,
    request: <TResult, TVariables extends Record<string, unknown>>(_document: TypedDocumentNode<TResult, TVariables>, variables: TVariables) => {
      hub.asked.push(variables);
      if (hub.fail !== null) return Promise.resolve({ ok: false, error: hub.fail } as HubResult<TResult>);
      return Promise.resolve({ ok: true, value: { agentUsage: hub.answer, everyone: { devices: hub.everyone } } } as unknown as HubResult<TResult>);
    },
  };
  return hub;
}
