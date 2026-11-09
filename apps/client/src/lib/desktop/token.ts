import { invoke, isTauri } from '@tauri-apps/api/core';
import { parseHubAddress, type HubAddress } from '$lib/hub/hub';

/** Why the key store did not answer. `not-desktop` is the client's own; the rest are the shell's (CLI-003). */
export type AccessTokenProblem = 'unavailable' | 'denied' | 'failed' | 'empty' | 'origin' | 'not-desktop';

const MESSAGES: Record<AccessTokenProblem, string> = {
  unavailable: 'The key store is not available.',
  denied: 'The key store refused access.',
  failed: 'The key store failed.',
  empty: 'The access token is empty.',
  origin: 'The hub address is not valid for the key store.',
  'not-desktop': 'The access token is kept by the desktop app only.',
};

const SHELL_CODES: readonly AccessTokenProblem[] = ['unavailable', 'denied', 'failed', 'empty', 'origin'];

/** A key-store failure. The message is fixed text chosen here; nothing from the shell's error is kept (LOG-007). */
export class AccessTokenError extends Error {
  constructor(readonly problem: AccessTokenProblem) {
    super(MESSAGES[problem]);
    this.name = 'AccessTokenError';
  }
}

/** The shell's typed error code, or `failed` for any shape it does not recognise. */
function problemOf(reason: unknown): AccessTokenProblem {
  const code = typeof reason === 'object' && reason !== null && 'code' in reason ? reason.code : undefined;
  return SHELL_CODES.find((known) => known === code) ?? 'failed';
}

async function invokeKeyStore(command: string, args: Record<string, unknown>): Promise<unknown> {
  try {
    return await invoke(command, args);
  } catch (reason) {
    // No `cause`: the shell's error could carry the token (LOG-007).
    throw new AccessTokenError(problemOf(reason));
  }
}

function requireDesktop(): void {
  if (!isTauri()) throw new AccessTokenError('not-desktop');
}

/**
 * The key for `address`: its origin, once the whole address is what `parseHubAddress` makes of that origin. A hub's token
 * goes only to that hub's endpoints, so a hand-built address cannot pair one hub's key with another hub's `http` or
 * `socket` (CLI-002, CLI-003).
 */
function keyOf(address: HubAddress): string {
  const parsed = parseHubAddress(address.origin);
  const same =
    parsed.ok && parsed.value.origin === address.origin && parsed.value.http === address.http && parsed.value.socket === address.socket;
  if (!same) throw new AccessTokenError('origin');
  return address.origin;
}

/**
 * The access token stored for `address.origin`, or `null` when none is stored for it or the app is not the desktop app.
 * A token is never sent to another hub (CLI-002). A key-store failure rejects with `AccessTokenError`, which
 * `createHub` shows as an unreadable token.
 */
export async function readAccessToken(address: HubAddress): Promise<string | null> {
  // In a browser a spoofed `isTauri()` only reaches `failed` (no shell, nothing to leak).
  if (!isTauri()) return null;
  const token = await invokeKeyStore('access_token_read', { origin: keyOf(address) });
  if (token === null) return null;
  if (typeof token !== 'string') throw new AccessTokenError('failed');
  return token;
}

/**
 * Keep `token` in the OS key store for `address.origin` (CLI-003). Write with the `hub.address` of the Hub that ran the
 * login mutation. Rejects with `AccessTokenError`: `not-desktop` off desktop, `origin` for a non-canonical address.
 */
export async function writeAccessToken(address: HubAddress, token: string): Promise<void> {
  requireDesktop();
  await invokeKeyStore('access_token_write', { origin: keyOf(address), token });
}

/** Remove the access token stored for `address.origin`, for log out (CLI-003). Rejects as `writeAccessToken` does. */
export async function clearAccessToken(address: HubAddress): Promise<void> {
  requireDesktop();
  await invokeKeyStore('access_token_clear', { origin: keyOf(address) });
}

/** This computer's builder token for `address.origin`, or `null` when none is stored or off desktop (BLD-023). */
export async function readBuilderToken(address: HubAddress): Promise<string | null> {
  if (!isTauri()) return null;
  const token = await invokeKeyStore('builder_token_read', { origin: keyOf(address) });
  if (token === null) return null;
  if (typeof token !== 'string') throw new AccessTokenError('failed');
  return token;
}

/** Keep the builder token `enrolBuilder` returned in the OS key store, never in `localStorage` (BLD-023). */
export async function writeBuilderToken(address: HubAddress, token: string): Promise<void> {
  requireDesktop();
  await invokeKeyStore('builder_token_write', { origin: keyOf(address), token });
}

/** Remove the builder token when the operator turns building off (BLD-025). */
export async function clearBuilderToken(address: HubAddress): Promise<void> {
  requireDesktop();
  await invokeKeyStore('builder_token_clear', { origin: keyOf(address) });
}
