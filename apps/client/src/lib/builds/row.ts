import type { BuilderTool, ToolCheck } from '$lib/desktop/builder/shell';
import type { BuilderPlatform } from './api';
import type { BuilderStatus, StepName } from './loop';

/** Each tool the Builds row lists under "Missing:" (BLD-028). */
export const TOOL_NAMES: Readonly<Record<BuilderTool, string>> = {
  git: 'git',
  bun: 'Bun',
  rust: 'Rust (cargo)',
  'cargo-nextest': 'cargo-nextest',
  postgres: 'PostgreSQL 17',
  'tauri-packages': 'the Tauri system packages',
  'updater-public-key': "this app's updater public key (an app built with the placeholder cannot build releases)",
  'signing-identity': 'the Developer ID certificate in the login keychain',
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
  upload: 'uploading',
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

/** The builder's line of the row: what it is doing, or what turning it on or off is doing (BLD-020). */
export function builderText(enrolled: boolean | null, busy: boolean, status: BuilderStatus): string {
  if (busy) return enrolled === true ? 'Turning off…' : 'Enrolling…';
  return enrolled === true ? statusText(status) : 'Off.';
}
