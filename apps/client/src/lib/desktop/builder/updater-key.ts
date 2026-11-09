import { invoke, isTauri } from '@tauri-apps/api/core';

/** What the shell says about the updater key; the key itself never reaches the web view (BLD-045). */
export interface UpdaterKeyState {
  readonly present: boolean;
  /** Whether the key is open in this app session, so the builder may sign (D-19). */
  readonly unlocked: boolean;
  /** The public key line (`RWQ...`) of the stored key. */
  readonly publicKey: string | null;
  /** Whether that is the public key this app checks updates with; `null` without a key. */
  readonly matchesApp: boolean | null;
}

const CODES = [
  'cancelled',
  'file',
  'format',
  'password-needed',
  'password',
  'new-password-needed',
  'missing',
  'locked',
  'caller',
  'unavailable',
  'denied',
  'failed',
] as const;

export type UpdaterKeyProblem = (typeof CODES)[number] | 'not-desktop';

const MESSAGES: Record<UpdaterKeyProblem, string> = {
  cancelled: 'No key file was chosen.',
  file: 'The key file could not be read.',
  format: 'The file is not an updater key.',
  'password-needed': 'The key file is encrypted. Type its password, then import it again.',
  password: 'The password does not open the key.',
  'new-password-needed': 'The key file has no password. Set a new password for it, then import it again.',
  missing: 'No updater key is stored on this computer.',
  locked: 'The updater key is locked. Unlock it with its password.',
  caller: 'Only the main window may use the updater key.',
  unavailable: 'The key store is not available.',
  denied: 'The key store refused access.',
  failed: 'The updater key could not be read.',
  'not-desktop': 'The updater key is kept by the desktop app only.',
};

/** An updater key failure, with fixed text chosen here (LOG-007). */
export class UpdaterKeyError extends Error {
  constructor(readonly problem: UpdaterKeyProblem) {
    super(MESSAGES[problem]);
    this.name = 'UpdaterKeyError';
  }
}

function problemOf(reason: unknown): UpdaterKeyProblem {
  const code = typeof reason === 'object' && reason !== null && 'code' in reason ? reason.code : undefined;
  return CODES.find((known) => known === code) ?? 'failed';
}

/** The shell's answer as an `UpdaterKeyState`; anything else is `failed`. */
export function updaterKeyStateFrom(answer: unknown): UpdaterKeyState {
  if (typeof answer !== 'object' || answer === null) throw new UpdaterKeyError('failed');
  const { present, unlocked, publicKey, matchesApp } = answer as Record<string, unknown>;
  const key = publicKey === null || typeof publicKey === 'string';
  const matches = matchesApp === null || typeof matchesApp === 'boolean';
  if (typeof present !== 'boolean' || typeof unlocked !== 'boolean' || !key || !matches) {
    throw new UpdaterKeyError('failed');
  }
  return { present, unlocked, publicKey, matchesApp };
}

async function answerOf(command: string, args: Record<string, unknown>): Promise<unknown> {
  if (!isTauri()) throw new UpdaterKeyError('not-desktop');
  try {
    return await invoke(command, args);
  } catch (reason) {
    throw new UpdaterKeyError(problemOf(reason));
  }
}

async function call(command: string, args: Record<string, unknown>): Promise<UpdaterKeyState> {
  return updaterKeyStateFrom(await answerOf(command, args));
}

function given(password: string | null): string | null {
  return password === '' ? null : password;
}

/** Whether this computer holds the updater key (BLD-020). */
export function readUpdaterKey(): Promise<UpdaterKeyState> {
  return call('updater_key_read', {});
}

/**
 * Asks the shell to open a file dialog, read the key file the operator chooses, open it with `password` and keep it
 * sealed in the key store (BLD-045, UPD-012, D-19). A file without a password is sealed with `newPassword`. The key is
 * then unlocked for this app session. The web view never sees the path or the key.
 */
export function importUpdaterKey(password: string | null, newPassword: string | null): Promise<UpdaterKeyState> {
  return call('updater_key_import', { password: given(password), newPassword: given(newPassword) });
}

/** Opens the stored key with `password` for this app session (D-19). */
export function unlockUpdaterKey(password: string): Promise<UpdaterKeyState> {
  return call('updater_key_unlock', { password });
}

/** Drops the opened key from the shell's memory (D-19); the sealed key stays stored. */
export async function lockUpdaterKey(): Promise<void> {
  await answerOf('updater_key_lock', {});
}
