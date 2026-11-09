import type { BuilderTool, ToolCheck } from '$lib/desktop/builder/shell';
import type { UpdaterKeyState } from '$lib/desktop/builder/updaterKey';
import type { BuilderPlatform } from './api';
import type { BuilderStatus, StepName } from './loop';

/** Each tool the Builds row names when it is missing (BLD-028). */
export const TOOL_NAMES: Readonly<Record<BuilderTool, string>> = {
  git: 'git',
  bun: 'Bun',
  rust: 'Rust (cargo)',
  postgres: 'PostgreSQL 17',
  'tauri-packages': 'The Tauri system packages',
  'updater-key': 'The updater key',
};

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

/** The updater key line of the row (BLD-020, BLD-045). */
export function keyText(key: UpdaterKeyState | null): string {
  if (key === null) return '';
  if (!key.present) return 'No updater key is stored on this computer.';
  if (key.matchesApp === false) return 'The stored updater key does not match the public key of this app.';
  return 'The updater key is stored on this computer.';
}
