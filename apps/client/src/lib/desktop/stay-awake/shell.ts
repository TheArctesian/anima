import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/** The three states of the control (AWK-001). */
export type Level = 'off' | 'awake' | 'lid-closed';

/** Why the computer is held at a lower level than the operator chose (AWK-006, AWK-008). */
export type Pause = 'low-battery' | 'lid-not-ready' | 'unavailable';

/** Whether this computer can keep awake with the lid closed (AWK-003). */
export type Lid = 'ready' | 'needs-set-up' | 'unsupported';

/** The stay-awake control of this computer, as the desktop shell reports it. */
export interface StayAwakeStatus {
  /** The operator's choice. */
  readonly level: Level;
  /** On battery below this percentage the control is held off; `null` is no floor (AWK-006). */
  readonly batteryFloor: number | null;
  /** The level the computer is held at now. */
  readonly held: Level;
  readonly pause: Pause | null;
  readonly lid: Lid;
}

/** The shell's error codes. Any other answer is `failed`. */
export type StayAwakeProblem = 'caller' | 'unsaved' | 'unsupported' | 'refused' | 'failed';

export type StayAwakeResult =
  | { readonly ok: true; readonly status: StayAwakeStatus }
  | { readonly ok: false; readonly problem: StayAwakeProblem };

const LEVELS: readonly unknown[] = ['off', 'awake', 'lid-closed'] satisfies readonly Level[];
const PAUSES: readonly unknown[] = ['low-battery', 'lid-not-ready', 'unavailable'] satisfies readonly Pause[];
const LIDS: readonly unknown[] = ['ready', 'needs-set-up', 'unsupported'] satisfies readonly Lid[];
const PROBLEMS: readonly unknown[] = ['caller', 'unsaved', 'unsupported', 'refused', 'failed'] satisfies readonly StayAwakeProblem[];

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null;
const isLevel = (value: unknown): value is Level => LEVELS.includes(value);
const isPause = (value: unknown): value is Pause => PAUSES.includes(value);
const isLid = (value: unknown): value is Lid => LIDS.includes(value);
const isProblem = (value: unknown): value is StayAwakeProblem => PROBLEMS.includes(value);
const isFloor = (value: unknown): value is number | null => value === null || (typeof value === 'number' && Number.isInteger(value));

/** The status the shell answered, or `null` for anything that is not one. */
export function statusOf(value: unknown): StayAwakeStatus | null {
  if (!isRecord(value)) return null;
  const { level, batteryFloor, held, pause, lid } = value;
  if (!isLevel(level) || !isLevel(held) || !isLid(lid) || !isFloor(batteryFloor)) return null;
  if (pause !== null && !isPause(pause)) return null;
  return { level, batteryFloor, held, pause, lid };
}

async function call(command: string, args?: Record<string, unknown>): Promise<StayAwakeResult> {
  if (!isTauri()) return { ok: false, problem: 'unsupported' };
  try {
    const status = statusOf(await invoke(command, args));
    return status === null ? { ok: false, problem: 'failed' } : { ok: true, status };
  } catch (reason) {
    const code = isRecord(reason) ? reason.code : undefined;
    return { ok: false, problem: isProblem(code) ? code : 'failed' };
  }
}

/** The state of the control now. */
export function readStayAwake(): Promise<StayAwakeResult> {
  return call('stay_awake_read');
}

/** Chooses a level; the shell saves it (AWK-001). */
export function chooseLevel(level: Level): Promise<StayAwakeResult> {
  return call('stay_awake_choose', { level });
}

/** Sets the battery floor in percent, or removes it with `null` (AWK-006). */
export function setBatteryFloor(floor: number | null): Promise<StayAwakeResult> {
  return call('stay_awake_floor', { floor });
}

/** Starts the set-up step of the lid-closed level; macOS asks for an administrator (AWK-003). */
export function setUpLid(): Promise<StayAwakeResult> {
  return call('stay_awake_lid_set_up');
}

/** Removes the set-up step; macOS asks for an administrator. */
export function removeLidSetUp(): Promise<StayAwakeResult> {
  return call('stay_awake_lid_remove');
}

/**
 * Hears each new state: a change from the tray item, or the battery (AWK-004, AWK-006). Resolves with the function
 * that stops listening. A payload that is not a status is dropped.
 */
export async function onStayAwake(listener: (status: StayAwakeStatus) => void): Promise<() => void> {
  if (!isTauri()) return () => undefined;
  return listen('stay-awake', (event) => {
    const status = statusOf(event.payload);
    if (status !== null) listener(status);
  });
}
