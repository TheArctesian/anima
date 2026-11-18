import type { HubError, HubResult } from '$lib/hub/hub';
import type { Container, ContainerCalls, ContainerLog } from './api';

// Test support: containers in the hub's shape, and calls that answer with them and record what the screen asked.

export const APP_ID = `4f5c1d2e3a4b${'0'.repeat(52)}`;

export function container(change: Partial<Container>): Container {
  return { id: APP_ID, name: 'tracks-app-1', image: 'example/tracks:0.36', state: 'RUNNING', status: 'Up 3 hours (healthy)', health: 'HEALTHY', ports: ['0.0.0.0:3000->3000/tcp'], createdAt: '2026-11-18T08:00:00Z', composeProject: 'tracks', composeService: 'app', ...change };
}

export const CONTAINERS: readonly Container[] = [
  container({ id: 'c'.repeat(64), name: 'recorder', image: 'example/recorder:1.0', status: 'Up 9 days', health: 'NONE', ports: [], composeProject: null, composeService: null }),
  container({ id: 'b'.repeat(64), name: 'tracks-worker-1', state: 'EXITED', status: 'Exited (1) 2 hours ago', health: 'NONE', ports: [], composeService: 'worker' }),
  container({}),
  container({ id: 'a'.repeat(64), name: 'ollama', image: 'example/ollama:0.9', status: 'Up 2 minutes (unhealthy)', health: 'UNHEALTHY', ports: [], composeProject: 'ai', composeService: 'ollama' }),
];

export const LOG: ContainerLog = {
  lines: [
    { at: '2026-11-18T18:00:00+00:00', text: 'listening on :3000' },
    { at: null, text: '<b>ready</b>' },
  ],
  cut: false,
};

export interface FakeCalls extends ContainerCalls {
  /** Each log read, in order: the container and the number of lines. */
  readonly logsAsked: [string, number][];
  listsAsked: number;
  /** What the next call answers instead of its answer. */
  fail: HubError | null;
  containers: readonly Container[];
  logAnswer: ContainerLog;
}

export function fakeCalls(containers: readonly Container[] = CONTAINERS): FakeCalls {
  const answer = <T>(value: T): Promise<HubResult<T>> => Promise.resolve(calls.fail === null ? { ok: true, value } : { ok: false, error: calls.fail });
  const calls: FakeCalls = {
    logsAsked: [],
    listsAsked: 0,
    fail: null,
    containers,
    logAnswer: LOG,
    list: () => {
      calls.listsAsked += 1;
      return answer(calls.containers);
    },
    log: (containerId, lines) => {
      calls.logsAsked.push([containerId, lines]);
      return answer(calls.logAnswer);
    },
  };
  return calls;
}
