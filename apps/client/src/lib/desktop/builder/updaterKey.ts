import { invoke, isTauri } from '@tauri-apps/api/core';

/** What the shell says about the updater key; the key itself never reaches the web view (BLD-045). */
export interface UpdaterKeyState {
  readonly present: boolean;
  /** The public key line (`RWQ...`) of the stored key. */
  readonly publicKey: string | null;
  /** Whether that is the public key this app checks updates with; `null` without a key. */
  readonly matchesApp: boolean | null;
}

const CODES = ['cancelled', 'file', 'format', 'password-needed', 'password', 'missing', 'unavailable', 'denied', 'failed'] as const;

export type UpdaterKeyProblem = (typeof CODES)[number] | 'not-desktop';

const MESSAGES: Record<UpdaterKeyProblem, string> = {
  cancelled: 'No key file was chosen.',
  file: 'The key file could not be read.',
  format: 'The file is not an updater key.',
  'password-needed': 'The key file is encrypted. Type its password, then import it again.',
  password: 'The password does not open the key file.',
  missing: 'No updater key is stored on this computer.',
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
  const { present, publicKey, matchesApp } = answer as Record<string, unknown>;
  const key = publicKey === null || typeof publicKey === 'string';
  const matches = matchesApp === null || typeof matchesApp === 'boolean';
  if (typeof present !== 'boolean' || !key || !matches) throw new UpdaterKeyError('failed');
  return { present, publicKey, matchesApp };
}

async function call(command: string, args: Record<string, unknown>): Promise<UpdaterKeyState> {
  if (!isTauri()) throw new UpdaterKeyError('not-desktop');
  let answer: unknown;
  try {
    answer = await invoke(command, args);
  } catch (reason) {
    throw new UpdaterKeyError(problemOf(reason));
  }
  return updaterKeyStateFrom(answer);
}

/** Whether this computer holds the updater key (BLD-020). */
export function readUpdaterKey(): Promise<UpdaterKeyState> {
  return call('updater_key_read', {});
}

/**
 * Asks the shell to open a file dialog, read the key file the operator chooses, open it with `password` and keep it in
 * the key store (BLD-045, UPD-012). The web view never sees the path or the key.
 */
export function importUpdaterKey(password: string | null): Promise<UpdaterKeyState> {
  return call('updater_key_import', { password: password === '' ? null : password });
}
