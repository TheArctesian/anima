import type { Row } from '../fixtures';
import { deviceOf, type HubState } from '../model';

// The use of the AI agent programs, so the Agent usage page has something to show (AIS-020, AIS-021, AIS-024). A Mac with
// Claude Code and Codex on several days, models and repositories, one model with no price; a Linux box whose daemon is a system
// unit; a device with the counts turned off. The devices are the fixture devices by name: without them (an emptied fixture
// list) no daemon reports, which is the empty case. Every host and repository is an example one (CON-003).

const DAY_MS = 86_400_000;
const PRICES_AS_OF = '2026-11-01';

interface Price {
  /** US dollars for a million tokens: input, output, cache read, cache write. */
  readonly perMillion: readonly [number, number, number, number];
}

const PRICES: Readonly<Record<string, Price>> = {
  'claude-opus-4': { perMillion: [15, 75, 1.5, 18.75] },
  'claude-sonnet-4': { perMillion: [3, 15, 0.3, 3.75] },
  'gpt-5-codex': { perMillion: [1.25, 10, 0.125, 0] },
};

interface Day {
  readonly device: string;
  readonly source: 'CLAUDE_CODE' | 'CODEX';
  readonly model: string;
  readonly repository: string;
  readonly daysAgo: number;
  readonly sessions: number;
  readonly prompts: number;
  readonly toolCalls: number;
  /** Input, output, cache read, cache write. */
  readonly tokens: readonly [number, number, number, number];
}

const DAYS: readonly Day[] = [
  { device: 'mac-mini', source: 'CLAUDE_CODE', model: 'claude-opus-4', repository: 'github.com/example/anima', daysAgo: 0, sessions: 3, prompts: 41, toolCalls: 380, tokens: [18_300, 210_000, 2_200_000, 61_000] },
  { device: 'mac-mini', source: 'CLAUDE_CODE', model: 'claude-sonnet-4', repository: 'github.com/example/site', daysAgo: 0, sessions: 1, prompts: 9, toolCalls: 70, tokens: [4_100, 52_000, 480_000, 12_500] },
  { device: 'mac-mini', source: 'CODEX', model: 'gpt-5-codex', repository: 'github.com/example/anima', daysAgo: 1, sessions: 2, prompts: 14, toolCalls: 120, tokens: [950, 18_000, 640_000, 0] },
  { device: 'mac-mini', source: 'CLAUDE_CODE', model: 'claude-opus-4', repository: 'github.com/example/anima', daysAgo: 2, sessions: 4, prompts: 52, toolCalls: 510, tokens: [22_000, 300_000, 3_100_000, 80_000] },
  { device: 'mac-mini', source: 'CODEX', model: 'local-llama-7b', repository: 'github.com/example/scratch', daysAgo: 3, sessions: 2, prompts: 11, toolCalls: 25, tokens: [1_400_000, 90_000, 0, 0] },
  { device: 'mac-mini', source: 'CLAUDE_CODE', model: 'claude-sonnet-4', repository: 'github.com/example/site', daysAgo: 9, sessions: 2, prompts: 17, toolCalls: 140, tokens: [6_000, 61_000, 700_000, 20_000] },
  { device: 'mac-mini', source: 'CODEX', model: 'gpt-5-codex', repository: 'github.com/example/anima', daysAgo: 20, sessions: 1, prompts: 6, toolCalls: 44, tokens: [700, 9_000, 210_000, 0] },
  { device: 'mac-mini', source: 'CLAUDE_CODE', model: 'claude-opus-4', repository: 'github.com/example/anima', daysAgo: 60, sessions: 5, prompts: 66, toolCalls: 600, tokens: [30_000, 410_000, 4_000_000, 90_000] },
];

type Dimension = 'DEVICE' | 'SOURCE' | 'MODEL' | 'REPOSITORY' | 'DAY';

interface Args {
  readonly filter?: { deviceId?: string | null; source?: string | null; repository?: string | null; firstDay?: string | null; lastDay?: string | null } | null;
  readonly by?: readonly Dimension[];
}

const STATES: Readonly<Record<string, string>> = { 'mac-mini': 'READING', nas: 'SYSTEM_UNIT', 'gpu-box': 'TURNED_OFF' };

const dayOf = (moment: number): string => new Date(moment).toISOString().slice(0, 10);

function cost(model: string, tokens: Day['tokens']): number | null {
  const price = PRICES[model];
  return price === undefined ? null : tokens.reduce((sum, count, index) => sum + (count * (price.perMillion[index] ?? 0)) / 1e6, 0);
}

interface Sums {
  sessions: number;
  prompts: number;
  toolCalls: number;
  tokens: [number, number, number, number];
  cost: number | null;
  unpriced: number;
}

function add(sums: Sums, day: Day): void {
  sums.sessions += day.sessions;
  sums.prompts += day.prompts;
  sums.toolCalls += day.toolCalls;
  day.tokens.forEach((count, index) => (sums.tokens[index] = (sums.tokens[index] ?? 0) + count));
  const priced = cost(day.model, day.tokens);
  if (priced === null) sums.unpriced += day.tokens.reduce((sum, count) => sum + count, 0);
  else sums.cost = (sums.cost ?? 0) + priced;
}

function sumsView({ sessions, prompts, toolCalls, tokens, cost: estimate, unpriced }: Sums): Row {
  const [inputTokens, outputTokens, cacheReadTokens, cacheWriteTokens] = tokens;
  return { sessions, prompts, toolCalls, inputTokens, outputTokens, cacheReadTokens, cacheWriteTokens, estimatedCostUsd: estimate, unpricedTokens: unpriced };
}

/** The Query fields `agentUsage` and `agentSessions` of the synthetic hub. */
export function agentUsageFields(state: HubState): Record<string, unknown> {
  return {
    agentUsage: ({ filter, by = [] }: Args): Row => {
      const now = state.now().getTime();
      const lastDay = filter?.lastDay ?? dayOf(now);
      const firstDay = filter?.firstDay ?? dayOf(Date.parse(`${lastDay}T00:00:00Z`) - 29 * DAY_MS);
      const devices = state.fixtures.devices.filter((device) => STATES[String(device.name)] !== undefined && (filter?.deviceId == null || device.id === filter.deviceId));
      const idOf = (name: string): string => String(devices.find((device) => device.name === name)?.id ?? '');
      const admitted = DAYS.filter((day) => {
        const when = dayOf(now - day.daysAgo * DAY_MS);
        return idOf(day.device) !== '' && when >= firstDay && when <= lastDay && (filter?.source == null || day.source === filter.source) && (filter?.repository == null || day.repository === filter.repository);
      });
      const groups = new Map<string, { day: Day; sums: Sums }>();
      const total: Sums = { sessions: 0, prompts: 0, toolCalls: 0, tokens: [0, 0, 0, 0], cost: null, unpriced: 0 };
      for (const day of admitted) {
        const key = by.map((dimension) => ({ DEVICE: day.device, SOURCE: day.source, MODEL: day.model, REPOSITORY: day.repository, DAY: String(day.daysAgo) })[dimension]).join('|');
        const group = groups.get(key) ?? { day, sums: { sessions: 0, prompts: 0, toolCalls: 0, tokens: [0, 0, 0, 0] as [number, number, number, number], cost: null, unpriced: 0 } };
        add(group.sums, day);
        add(total, day);
        groups.set(key, group);
      }
      const rows = [...groups.values()]
        .toSorted((a, b) => a.day.daysAgo - b.day.daysAgo || a.day.device.localeCompare(b.day.device) || a.day.source.localeCompare(b.day.source) || a.day.model.localeCompare(b.day.model))
        .map(({ day, sums }) => ({
          device: by.includes('DEVICE') ? deviceOf(state, idOf(day.device)) : null,
          source: by.includes('SOURCE') ? day.source : null,
          model: by.includes('MODEL') ? day.model : null,
          repository: by.includes('REPOSITORY') ? day.repository : null,
          day: by.includes('DAY') ? dayOf(now - day.daysAgo * DAY_MS) : null,
          ...sumsView(sums),
        }));
      return {
        firstDay,
        lastDay,
        rows,
        rowsLeftOut: 0,
        total: { device: null, source: null, model: null, repository: null, day: null, ...sumsView(total) },
        devices: devices
          .toSorted((a, b) => String(a.name).localeCompare(String(b.name)))
          .map((device) => ({ device: deviceOf(state, String(device.id)), state: STATES[String(device.name)] })),
        pricesAsOf: PRICES_AS_OF,
        accounts: accountsOf(state, devices.map((device) => String(device.name))),
      };
    },
    agentSessions: (args: Args & { first?: number }): Row => sessionsOf(state, args),
  };
}

/**
 * The accounts of the programs, as keys only (AIS-022 as R-313 changed it): one Codex account that the mac-mini and the
 * gpu-box share, near its weekly limit, and the Claude Code account of the mac-mini, which reports no limit.
 */
function accountsOf(state: HubState, names: readonly string[]): Row[] {
  const now = state.now().getTime();
  const all = state.fixtures.devices;
  const devicesOf = (wanted: readonly string[]) =>
    wanted.filter((name) => names.includes(name)).map((name) => deviceOf(state, String(all.find((device) => device.name === name)?.id ?? '')));
  const accounts = [
    {
      key: '3f9a12c4'.padEnd(64, '7'),
      shortKey: '3f9a12c4',
      source: 'CODEX',
      plan: 'Pro',
      reportedAt: new Date(now - 2 * 60_000).toISOString(),
      devices: devicesOf(['mac-mini', 'gpu-box']),
      limits: [
        { window: 'five_hour', usedPercent: 35, resetsAt: new Date(now + 3 * 3_600_000).toISOString(), reached: false },
        { window: 'seven_day', usedPercent: 82, resetsAt: new Date(now + 4 * DAY_MS).toISOString(), reached: false },
      ],
    },
    { key: 'a71c0e5b'.padEnd(64, '7'), shortKey: 'a71c0e5b', source: 'CLAUDE_CODE', plan: 'Max', reportedAt: new Date(now - 12 * 60_000).toISOString(), devices: devicesOf(['mac-mini']), limits: [] },
  ];
  return accounts.filter((account) => account.devices.length > 0);
}

/** The Query field `agentSessions`: the entries of DAYS split into their sessions (AIS-025, AIS-026). */
function sessionsOf(state: HubState, { filter, first = 50 }: Args & { first?: number }): Row {
  const now = state.now().getTime();
  const lastDay = filter?.lastDay ?? dayOf(now);
  const firstDay = filter?.firstDay ?? dayOf(Date.parse(`${lastDay}T00:00:00Z`) - 29 * DAY_MS);
  const devices = state.fixtures.devices.filter((device) => STATES[String(device.name)] !== undefined && (filter?.deviceId == null || device.id === filter.deviceId));
  const idOf = (name: string): string => String(devices.find((device) => device.name === name)?.id ?? '');
  // Each entry of DAYS is split into its sessions, each with an equal part of the counts. The newest session of today was
  // last used 10 minutes ago, so the page has one recent session (AIS-026).
  const sessions = DAYS.flatMap((day) => {
    const when = dayOf(now - day.daysAgo * DAY_MS);
    const admitted = idOf(day.device) !== '' && when >= firstDay && when <= lastDay && (filter?.source == null || day.source === filter.source) && (filter?.repository == null || day.repository === filter.repository);
    if (!admitted) return [];
    return Array.from({ length: day.sessions }, (_, index) => {
      const lastUse = now - day.daysAgo * DAY_MS - (index * 3 + (day.daysAgo === 0 && index === 0 ? 0 : 1)) * 3_600_000 - 600_000;
      const part = (count: number): number => Math.round(count / day.sessions);
      const tokens = day.tokens.map(part) as [number, number, number, number];
      const sums: Sums = { sessions: 1, prompts: 0, toolCalls: 0, tokens: [0, 0, 0, 0], cost: null, unpriced: 0 };
      add(sums, { ...day, sessions: 1, prompts: part(day.prompts), toolCalls: part(day.toolCalls), tokens });
      // The field `sessions` of the sums is no field of AgentSession; GraphQL leaves it out of the answer.
      const view = sumsView(sums);
      return {
        id: `${day.device}-${String(day.daysAgo)}-${day.model}-${String(index)}`,
        device: deviceOf(state, idOf(day.device)),
        source: day.source,
        model: day.model,
        repository: day.repository,
        startedAt: new Date(lastUse - 2 * 3_600_000).toISOString(),
        lastUseAt: new Date(lastUse).toISOString(),
        recent: now - lastUse <= 30 * 60_000,
        ...view,
      };
    });
  }).toSorted((a, b) => b.lastUseAt.localeCompare(a.lastUseAt));
  return { firstDay, lastDay, sessions: sessions.slice(0, first), sessionsLeftOut: Math.max(0, sessions.length - first) };
}
