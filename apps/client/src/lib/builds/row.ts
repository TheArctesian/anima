import type { BuilderTool, ToolCheck } from '$lib/desktop/builder/shell';
import type { UpdaterKeyState } from '$lib/desktop/builder/updater-key';
import type { BuilderPlatform } from './api';
import type { BuilderStatus, StepName } from './loop';

/** A tool that is not there; a locked updater key is there, and the key line says so. */
export type MissingTool = Exclude<BuilderTool, 'updater-key-locked'>;

/** Each tool the Builds row lists under "Missing:" (BLD-028). */
export const TOOL_NAMES: Readonly<Record<MissingTool, string>> = {
  git: 'git',
  bun: 'Bun',
  rust: 'Rust (cargo)',
  postgres: 'PostgreSQL 17',
  'tauri-packages': 'the Tauri system packages',
  'updater-key': 'the updater key',
};

/** The tools of `tools` that are not there, for the "Missing:" list. */
export function missingTools(tools: readonly BuilderTool[]): MissingTool[] {
  return tools.filter((tool): tool is MissingTool => tool !== 'updater-key-locked');
}

const OS_NAMES: Readonly<Record<BuilderPlatform['os'], string>> = { LINUX: 'Linux', MACOS: 'macOS' };
const ARCH_NAMES: Readonly<Record<BuilderPlatform['arch'], string>> = { X86_64: 'x86-64', AARCH64: 'Apple silicon (arm64)' };

/** A platform as the operator reads it, for example "Linux, x86-64". */
export function platformText(platform: BuilderPlatform): string {
  return `${OS_NAMES[platform.os]}, ${ARCH_NAMES[platform.arch]}`;
}

/** This computer's platform line of the row; `null` until the shell has answered. */
export function thisPlatformText(check: ToolCheck | null, failed: boolean): string {
  if (check !== null) return platformText(check);
  return failed ? 'This computer could not say which platform it is.' : '';
}

const STEP_NAMES: Readonly<Record<StepName, string>> = {
  checkout: 'checking out',
  gates: 'running the gates',
  bundle: 'bundling',
  sign: 'signing',
  verify: 'checking the signature',
};

const STATUS_TEXT: Readonly<Record<Exclude<BuilderStatus['kind'], 'building' | 'missing'>, string>> = {
  starting: 'Starting.',
  unsupported: 'This computer cannot build releases.',
  'tools-unknown': 'The tool check failed. The builder tries again shortly.',
  locked: 'The updater key is locked, so this computer claims no build.',
  waiting: 'Waiting for a release to build.',
  unreachable: 'The hub cannot be reached. The builder tries again shortly.',
  refused: 'The hub refused this builder. Turn building off, then on again.',
  stopped: 'Off.',
};

/** What the builder is doing, in one sentence (BLD-020). */
export function statusText(status: BuilderStatus): string {
  switch (status.kind) {
    case 'building':
      return `Building ${status.job.version}: ${STEP_NAMES[status.step]}.`;
    case 'missing':
      return 'A tool is missing, so this computer claims no build.';
    default:
      return STATUS_TEXT[status.kind];
  }
}

/** The builder's line of the row: what it is doing, or what turning it on or off is doing (BLD-020). */
export function builderText(enrolled: boolean | null, busy: boolean, status: BuilderStatus): string {
  if (busy) return enrolled === true ? 'Turning off…' : 'Enrolling…';
  return enrolled === true ? statusText(status) : 'Off.';
}

/** The updater key line of the row (BLD-020, BLD-045, D-19). */
export function keyText(key: UpdaterKeyState | null): string {
  if (key === null) return '';
  if (!key.present) return 'No updater key is stored on this computer.';
  if (key.matchesApp === false) return 'The stored updater key does not match the public key of this app.';
  if (!key.unlocked) return 'The updater key is stored and locked. Unlock it to build releases.';
  return 'The updater key is unlocked until the app quits or building is turned off.';
}

/**
 * The warning when other programs of the operator's user may read this app's memory, where the unlocked key is
 * (D-19); empty when they may not, or before the shell has said.
 */
export function memoryText(check: ToolCheck | null): string {
  if (check === null || check.memoryGuarded) return '';
  return "Other programs you run can read this app's memory, where the unlocked updater key is. Set kernel.yama.ptrace_scope to 1 to stop that.";
}
