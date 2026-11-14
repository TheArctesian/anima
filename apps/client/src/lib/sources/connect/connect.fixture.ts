import type { ConnectAttempt } from './api';

// Test support: a GitHub connection attempt in the hub's shape, and the hub's answers to the connection calls.

export function attempt(more: Partial<ConnectAttempt> = {}): ConnectAttempt {
  return {
    id: 'attempt-1',
    phase: 'WAITING_FOR_BROWSER',
    startedBy: 'USER',
    expiresAt: '2026-11-13T12:55:00Z',
    pageUrl: 'https://hub.example/connect/github',
    appSlug: null,
    ownerLogin: null,
    installUrl: null,
    appSettingsUrl: null,
    failure: null,
    message: null,
    account: null,
    ...more,
  };
}

/** What the fake hub holds for the connection: the open attempt and the one-time link it hands out. */
interface Held {
  connect: ConnectAttempt | null;
  readonly link: string;
}

/** The answer to a connection mutation, or undefined for any other operation. */
export function connectAnswer(held: Held, operation: string): Record<string, unknown> | undefined {
  if (operation === 'StartGithubConnect') {
    held.connect = attempt();
    return { startGithubConnect: held.connect };
  }
  if (operation === 'OpenGithubConnect') return { openGithubConnect: { url: held.link, expiresAt: '2026-11-13T12:05:00Z' } };
  if (operation === 'CancelGithubConnect') {
    held.connect = { ...(held.connect ?? attempt()), phase: 'CANCELLED' };
    return { cancelGithubConnect: held.connect };
  }
  return undefined;
}
