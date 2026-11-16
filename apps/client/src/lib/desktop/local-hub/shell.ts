import { invoke, isTauri } from '@tauri-apps/api/core';

/** What the hub group reports (LHB-010, `docs/setup/local-hub.md` Section 2). */
export type LocalHubState = 'starting' | 'ready' | 'blocked' | 'stopping' | 'stopped' | 'failed';

/** Why a hub is `blocked` (`docs/setup/local-hub.md` Section 4). */
export type BlockedReason = 'key-missing' | 'database-version' | 'database-in-use';

/** The link member of the group (F-6). */
export interface LinkMember {
  readonly state: 'starting' | 'up' | 'down';
  /** RFC 3339: when the link entered that state. */
  readonly since: string;
}

/** The hub that runs on this computer, as `local_hub_status` reports it. */
export interface LocalHubStatus {
  readonly installed: boolean;
  readonly running: boolean;
  readonly state?: LocalHubState;
  /** `host:port`, with `ready`. */
  readonly hub?: string;
  /** With `ready`: the database replayed its log after an unclean stop. */
  readonly recovered?: boolean;
  readonly reason?: BlockedReason;
  /** With `failed`: the program that ended. */
  readonly cause?: string;
  readonly link?: LinkMember;
}

/** The operator account of the local hub (FRN-003). */
export interface LocalAccount {
  readonly name: string;
  readonly totp: boolean;
}

/** A new TOTP secret and its `otpauth://` URI; the hub saves nothing yet (FRN-004). Both are secrets (LOG-007). */
export interface NewTotp {
  readonly secret: string;
  readonly uri: string;
}

/** What `local_hub_account_create` takes. Every field but `name` is a secret (LOG-007). */
export interface NewAccount {
  readonly name: string;
  readonly password: string;
  readonly secret: string;
  readonly code: string;
}

/** The shell's error codes; any other answer is `failed`. */
export type LocalHubProblem =
  | 'caller'
  | 'unavailable'
  | 'not-carried'
  | 'busy'
  | 'no-home'
  | 'timed-out'
  | 'failed'
  | 'argument'
  | 'refused'
  | 'no-key'
  | 'destination'
  | 'cancelled';

const MESSAGES: Readonly<Record<LocalHubProblem, string>> = {
  caller: 'Only the main window can do this.',
  unavailable: 'This computer cannot run the hub from the app.',
  'not-carried': 'This build does not carry the hub programs.',
  busy: 'Another change to the hub is running. Wait for it to finish.',
  'no-home': 'The app could not find your home folder.',
  'timed-out': 'The hub did not answer in time. Try again.',
  failed: 'The hub programs did not finish. The application log has the cause.',
  argument: 'The hub refused the form of this request.',
  refused: 'The hub refused the account. Check the password and the code.',
  'no-key': 'The hub has no key file yet.',
  destination: 'The key could not be written to that place. Choose another folder.',
  cancelled: 'Recovery was cancelled. Nothing changed.',
};

const PROBLEMS = new Set<string>(Object.keys(MESSAGES));

export type LocalHubResult<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly problem: LocalHubProblem; readonly message: string };

/** The fixed text for `problem`; the shell's own message is never shown (LOG-007). */
export function localHubMessage(problem: LocalHubProblem): string {
  return MESSAGES[problem];
}

function refused(problem: LocalHubProblem): LocalHubResult<never> {
  return { ok: false, problem, message: MESSAGES[problem] };
}

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null;
const STATES: readonly unknown[] = ['starting', 'ready', 'blocked', 'stopping', 'stopped', 'failed'] satisfies LocalHubState[];
const REASONS: readonly unknown[] = ['key-missing', 'database-version', 'database-in-use'] satisfies BlockedReason[];
const LINK_STATES: readonly unknown[] = ['starting', 'up', 'down'] satisfies LinkMember['state'][];
const optional = (value: unknown, check: (value: unknown) => boolean): boolean => value === undefined || value === null || check(value);
const isText = (value: unknown): value is string => typeof value === 'string';

function linkOf(value: unknown): LinkMember | undefined | null {
  if (value === undefined || value === null) return undefined;
  if (!isRecord(value) || !LINK_STATES.includes(value.state) || !isText(value.since)) return null;
  return { state: value.state as LinkMember['state'], since: value.since };
}

/** The status the shell answered, or `null` for anything that is not one. Absent fields stay absent. */
export function statusOf(value: unknown): LocalHubStatus | null {
  if (!isRecord(value) || typeof value.installed !== 'boolean' || typeof value.running !== 'boolean') return null;
  const { state, hub, recovered, reason, cause } = value;
  const fields = [optional(state, (v) => STATES.includes(v)), optional(hub, isText), optional(recovered, (v) => typeof v === 'boolean'), optional(reason, (v) => REASONS.includes(v)), optional(cause, isText)];
  const link = linkOf(value.link);
  if (fields.includes(false) || link === null) return null;
  const present = Object.entries({ state, hub, recovered, reason, cause, link }).filter(([, field]) => field !== undefined && field !== null);
  return { installed: value.installed, running: value.running, ...Object.fromEntries(present) } as LocalHubStatus;
}

function accountOf(value: unknown): LocalAccount | null | undefined {
  if (value === null) return null;
  if (!isRecord(value) || !isText(value.name) || typeof value.totp !== 'boolean') return undefined;
  return { name: value.name, totp: value.totp };
}

function totpOf(value: unknown): NewTotp | undefined {
  if (!isRecord(value) || !isText(value.secret) || !isText(value.uri) || !value.uri.startsWith('otpauth://')) return undefined;
  return { secret: value.secret, uri: value.uri };
}

const isNull = (value: unknown): null | undefined => (value === null ? null : undefined);
const isBool = (value: unknown): boolean | undefined => (typeof value === 'boolean' ? value : undefined);

/** Runs `command`; `shape` reads the answer and gives `undefined` for a wrong one. No argument or answer is logged (LOG-007). */
async function call<T>(command: string, shape: (value: unknown) => T | undefined, args?: Record<string, unknown>): Promise<LocalHubResult<T>> {
  if (!isTauri()) return refused('unavailable');
  try {
    const value = shape(await invoke(command, args));
    return value === undefined ? refused('failed') : { ok: true, value };
  } catch (reason) {
    // No `cause`: the shell's text is not kept (LOG-007).
    const code = isRecord(reason) ? reason.code : undefined;
    return refused(typeof code === 'string' && PROBLEMS.has(code) ? (code as LocalHubProblem) : 'failed');
  }
}

/** Whether this build carries the hub programs; false outside the desktop app or when the shell does not answer. */
export async function localHubCarried(): Promise<boolean> {
  const result = await call('local_hub_carried', isBool);
  return result.ok && result.value;
}

/** Installs the hub and starts it; the hub then starts in the background (FRN-002). */
export function installLocalHub(): Promise<LocalHubResult<null>> {
  return call('local_hub_install', () => null);
}

export function readLocalHub(): Promise<LocalHubResult<LocalHubStatus>> {
  return call('local_hub_status', (value) => statusOf(value) ?? undefined);
}

/** Starts the installed hub's programs together (FRN-006). */
export function startLocalHub(): Promise<LocalHubResult<null>> {
  return call('local_hub_start', () => null);
}

/** Stops them until Start or the next sign-in to the computer (FRN-006). */
export function stopLocalHub(): Promise<LocalHubResult<null>> {
  return call('local_hub_stop', () => null);
}

/** The operator account, or `null` before the first run made it. */
export function readLocalAccount(): Promise<LocalHubResult<LocalAccount | null>> {
  return call('local_hub_account', accountOf);
}

/** A new TOTP secret labelled with `name`; nothing is saved (FRN-004). */
export function newTotp(name: string): Promise<LocalHubResult<NewTotp>> {
  return call('local_hub_totp_new', totpOf, { name });
}

/**
 * Makes the account (FRN-003). When one exists the shell first asks the person at this computer in a native dialog,
 * which script cannot answer, and fails with `cancelled` when they decline (FRN-007, F-9). For 60 s after that it shows
 * no dialog and fails with `busy`. A name outside the rule fails with `argument`.
 */
export function createLocalAccount(account: NewAccount): Promise<LocalHubResult<null>> {
  return call('local_hub_account_create', isNull, { account });
}

/** A native save dialog for a copy of `vault.key`; `false` when the operator cancelled (2.8). */
export function saveHubKey(): Promise<LocalHubResult<boolean>> {
  return call('local_hub_key_save', isBool);
}
