import { invoke, isTauri } from '@tauri-apps/api/core';

/**
 * The `anima` command line tool on this computer's `PATH`, as the desktop shell reports it (INS-004 to INS-006). The
 * shell proposes the folder; the row shows it and sends it back unchanged when the operator confirms.
 */
export type CommandLineStatus =
  | { readonly state: 'unavailable'; readonly reason: CommandLineProblem }
  | {
      readonly state: 'not-installed';
      readonly folder: string;
      readonly path: string;
      readonly onPath: boolean;
      readonly create: boolean;
      readonly profileLine: string | null;
    }
  | { readonly state: 'installed'; readonly path: string }
  | { readonly state: 'elsewhere'; readonly folder: string; readonly path: string; readonly pointsAt: string };

/** The shell's error codes. Any other answer is `failed`. */
export type CommandLineProblem =
  | 'caller'
  | 'unsupported'
  | 'not-bundled'
  | 'disk-image'
  | 'translocated'
  | 'not-appimage'
  | 'unquotable'
  | 'no-home'
  | 'changed'
  | 'occupied'
  | 'unwritable'
  | 'failed';

const PROBLEMS: Readonly<Record<CommandLineProblem, string>> = {
  caller: 'Only the main window can install the command line tool.',
  unsupported: 'The command line tool cannot be installed on this system from the app.',
  'not-bundled': 'This build of the app carries no command line tool.',
  'disk-image': 'Anima is running from the disk image. Move Anima to Applications, open it from there, then install.',
  translocated: 'macOS runs Anima from a temporary place. Move Anima to Applications, open it from there, then install.',
  'not-appimage': 'Anima is not running from its AppImage, so there is nothing for a launcher to start.',
  unquotable: 'The path of the AppImage has a character a launcher cannot hold. Move the AppImage, then try again.',
  'no-home': 'Anima could not find your home folder.',
  changed: 'The proposed folder changed. Check the new path, then install.',
  occupied: 'Something that Anima did not make is already there, so Anima did not replace it.',
  unwritable: 'Anima could not write to the folder.',
  failed: 'The command line tool could not be changed.',
};

/** Fixed text for each problem; the shell's own message is never shown. */
export function problemText(problem: CommandLineProblem): string {
  return PROBLEMS[problem];
}

/** What blocks an install: the kind of entry, and where a link points. */
export interface Occupant {
  readonly kind: 'file' | 'folder' | 'link' | 'other';
  readonly linkTarget: string | null;
}

export type CommandLineResult =
  | { readonly ok: true; readonly status: CommandLineStatus }
  | { readonly ok: false; readonly problem: CommandLineProblem; readonly occupant: Occupant | null };

const isRecord = (value: unknown): value is Record<string, unknown> => typeof value === 'object' && value !== null;
const isProblem = (code: unknown): code is CommandLineProblem => typeof code === 'string' && Object.hasOwn(PROBLEMS, code);
const text = (value: unknown): value is string => typeof value === 'string';

function notInstalledOf(value: Record<string, unknown>): CommandLineStatus | null {
  const { folder, path, onPath, create, profileLine } = value;
  const line = profileLine === null || text(profileLine);
  return text(folder) && text(path) && typeof onPath === 'boolean' && typeof create === 'boolean' && line
    ? { state: 'not-installed', folder, path, onPath, create, profileLine: profileLine ?? null }
    : null;
}

function elsewhereOf(value: Record<string, unknown>): CommandLineStatus | null {
  const { folder, path, pointsAt } = value;
  return text(folder) && text(path) && text(pointsAt) ? { state: 'elsewhere', folder, path, pointsAt } : null;
}

/** The status the shell answered, or `null` for anything that is not one. */
export function statusOf(value: unknown): CommandLineStatus | null {
  if (!isRecord(value)) return null;
  switch (value.state) {
    case 'unavailable':
      return isProblem(value.reason) ? { state: 'unavailable', reason: value.reason } : null;
    case 'installed':
      return text(value.path) ? { state: 'installed', path: value.path } : null;
    case 'elsewhere':
      return elsewhereOf(value);
    case 'not-installed':
      return notInstalledOf(value);
    default:
      return null;
  }
}

function occupantOf(reason: Record<string, unknown>): Occupant | null {
  const { occupant: kind, linkTarget } = reason;
  if (kind !== 'file' && kind !== 'folder' && kind !== 'link' && kind !== 'other') return null;
  return { kind, linkTarget: text(linkTarget) ? linkTarget : null };
}

function failure(reason: unknown): CommandLineResult {
  if (!isRecord(reason) || !isProblem(reason.code)) return { ok: false, problem: 'failed', occupant: null };
  const problem = reason.code;
  return { ok: false, problem, occupant: problem === 'occupied' ? occupantOf(reason) : null };
}

async function call(command: string, args?: Record<string, string>): Promise<CommandLineResult> {
  if (!isTauri()) return { ok: false, problem: 'unsupported', occupant: null };
  try {
    const status = statusOf(await invoke(command, args));
    return status === null ? { ok: false, problem: 'failed', occupant: null } : { ok: true, status };
  } catch (reason) {
    return failure(reason);
  }
}

/** The tool's state on this computer (INS-006). */
export function readCommandLine(): Promise<CommandLineResult> {
  return call('command_line_read');
}

/** Installs the tool in `folder`, the folder the shell proposed and the operator confirmed (INS-004, INS-005). */
export function installCommandLine(folder: string): Promise<CommandLineResult> {
  return call('command_line_install', { folder });
}

/** Removes only the link or launcher an Anima app made (INS-006). */
export function removeCommandLine(): Promise<CommandLineResult> {
  return call('command_line_remove');
}
