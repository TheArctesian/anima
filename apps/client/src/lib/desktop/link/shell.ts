import { invoke, isTauri } from '@tauri-apps/api/core';

/** What a join gives: the new device key's short form and the local port of the device link (FRN-005, F-2). */
export interface Joined {
  readonly fingerprint: string;
  readonly port: number;
}

/** Why the device link is not up. */
export type LinkDownReason = 'port-taken' | 'ended' | 'repeated-failure' | 'not-carried' | 'no-port';

/** The device link of this computer, as `link_status` reports it. */
export interface DeviceLinkStatus {
  /** This device has joined a hub: its link folder is there. */
  readonly joined: boolean;
  readonly state: 'stopped' | 'starting' | 'up' | 'down';
  readonly port?: number;
  readonly publicKey?: string;
  readonly reason?: LinkDownReason;
  /** With `up` (LNK-006). */
  readonly path?: 'direct' | 'relayed' | 'none';
  readonly latencyMs?: number;
}

/** The shell's error codes; any other answer is `failed`. */
export type LinkProblem = 'caller' | 'not-carried' | 'busy' | 'no-home' | 'bad-invite' | 'no-answer' | 'refused' | 'folder' | 'failed' | 'unavailable';

const MESSAGES: Readonly<Record<LinkProblem, string>> = {
  caller: 'Only the main window can do this.',
  'not-carried': 'This build does not carry the link program.',
  busy: 'Another link change is running. Wait for it.',
  'no-home': 'The app could not find your home folder.',
  'bad-invite': 'Not an invite. Paste the whole text, starting with anima-invite.1.',
  'no-answer': 'The hub did not answer: the invite is used, expired or wrong, or the hub is off.',
  refused: 'The hub refused this device. Ask for a new invite.',
  folder: 'The app could not write the link folder.',
  failed: 'The link did not finish. See the application log.',
  unavailable: 'Only the desktop app can join a hub.',
};

const PROBLEMS = new Set<string>(Object.keys(MESSAGES));

export type LinkResult<T> = { readonly ok: true; readonly value: T } | { readonly ok: false; readonly problem: LinkProblem; readonly message: string };

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null;
const isPort = (value: unknown): value is number => Number.isInteger(value) && (value as number) > 0 && (value as number) < 65_536;
const STATES: readonly unknown[] = ['stopped', 'starting', 'up', 'down'] satisfies DeviceLinkStatus['state'][];
const REASONS: readonly unknown[] = ['port-taken', 'ended', 'repeated-failure', 'not-carried', 'no-port'] satisfies LinkDownReason[];
const PATHS: readonly unknown[] = ['direct', 'relayed', 'none'] satisfies NonNullable<DeviceLinkStatus['path']>[];
const absent = (value: unknown): boolean => value === undefined || value === null;

function refused(problem: LinkProblem): LinkResult<never> {
  return { ok: false, problem, message: MESSAGES[problem] };
}

/** The join answer, or `undefined` for anything that is not one. */
export function joinedOf(value: unknown): Joined | undefined {
  if (!isRecord(value) || typeof value.fingerprint !== 'string' || value.fingerprint === '' || !isPort(value.port)) return undefined;
  return { fingerprint: value.fingerprint, port: value.port };
}

/** The status the shell answered, or `undefined` for anything that is not one. Absent fields stay absent. */
export function linkStatusOf(value: unknown): DeviceLinkStatus | undefined {
  if (!isRecord(value) || typeof value.joined !== 'boolean' || !STATES.includes(value.state)) return undefined;
  const { port, reason, path, latencyMs, publicKey } = value;
  const valid = [absent(port) || isPort(port), absent(reason) || REASONS.includes(reason), absent(path) || PATHS.includes(path), absent(latencyMs) || typeof latencyMs === 'number', absent(publicKey) || (typeof publicKey === 'string' && publicKey !== '')];
  if (valid.includes(false)) return undefined;
  const present = Object.entries({ port, reason, path, latencyMs, publicKey }).filter(([, field]) => !absent(field));
  return { joined: value.joined, state: value.state, ...Object.fromEntries(present) } as DeviceLinkStatus;
}

/** Runs `command`; `shape` reads the answer. The invite is never logged or kept in an error (LOG-007). */
async function call<T>(command: string, shape: (value: unknown) => T | undefined, args?: Record<string, unknown>): Promise<LinkResult<T>> {
  if (!isTauri()) return refused('unavailable');
  try {
    const value = shape(await invoke(command, args));
    return value === undefined ? refused('failed') : { ok: true, value };
  } catch (reason) {
    const code = isRecord(reason) ? reason.code : undefined;
    return refused(typeof code === 'string' && PROBLEMS.has(code) ? (code as LinkProblem) : 'failed');
  }
}

/** Joins this device to the hub of `invite` with a new key, and starts the device link (FRN-005, F-7). */
export function joinLink(invite: string): Promise<LinkResult<Joined>> {
  return call('link_join', joinedOf, { invite });
}

export function readLink(): Promise<LinkResult<DeviceLinkStatus>> {
  return call('link_status', linkStatusOf);
}

/** Stops the device link and removes its key: this device leaves its hub. */
export function leaveLink(): Promise<LinkResult<null>> {
  return call('link_leave', () => null);
}
