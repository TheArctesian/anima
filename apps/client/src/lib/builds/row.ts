import type { BuilderTool, ToolCheck } from '$lib/desktop/builder/shell';
import type { BuilderPlatform } from './api';
import type { BuilderStatus, StepName } from './loop';

/** Each tool the Builds row lists under "Missing:" (BLD-028). */
export const TOOL_NAMES: Readonly<Record<BuilderTool, string>> = {
  git: 'git',
  bun: 'Bun',
  rust: 'Rust (cargo)',
  'cargo-nextest': 'cargo-nextest',
  go: 'Go (for the gates of link/)',
  postgres: 'PostgreSQL 17',
  'tauri-packages': 'the Tauri system packages',
  node: 'Node.js for the client tests',
  'go-race-compiler': 'MinGW GCC for the Go race tests',
  'tree-tools': 'the tools that build the PostgreSQL tree (make, bison, flex, patchelf, readelf, curl and the zlib headers in /usr)',
  'updater-public-key': "this app's updater public key (an app built with the placeholder cannot build releases)",
  'signing-identity': 'the Developer ID certificate in the login keychain',
  'rust-target-aarch64-apple-darwin':
    'the Rust target aarch64-apple-darwin (rustup target add aarch64-apple-darwin)',
  'rust-target-x86_64-apple-darwin':
    'the Rust target x86_64-apple-darwin (rustup target add x86_64-apple-darwin)',
};

const OS_NAMES: Readonly<Record<BuilderPlatform['os'], string>> = { LINUX: 'Linux', MACOS: 'macOS', WINDOWS: 'Windows' };
const ARCH_NAMES: Readonly<Record<BuilderPlatform['arch'], string>> = { X86_64: 'x86-64', AARCH64: 'Apple silicon (arm64)' };

/** A platform as the operator reads it, for example "Linux, x86-64". */
export function platformText(platform: BuilderPlatform): string {
  return `${OS_NAMES[platform.os]}, ${ARCH_NAMES[platform.arch]}`;
}

/** This computer's platform line of the row; `null` until the shell has answered. */
export function thisPlatformText(check: ToolCheck | null, failed: boolean): string {
  if (check !== null) return platformText(check);
  return failed ? 'Platform not detected.' : '';
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
  'tools-unknown': 'Tool check failed. Retrying shortly.',
  waiting: 'Waiting for a release.',
  unreachable: 'Hub unreachable. Retrying shortly.',
  refused: 'The hub refused this builder. Turn building off, then on.',
  stopped: 'Off.',
};

/** What the builder is doing, in one sentence (BLD-020). */
export function statusText(status: BuilderStatus): string {
  switch (status.kind) {
    case 'building':
      return `Building ${status.job.version}: ${STEP_NAMES[status.step]}.`;
    case 'missing':
      return 'A tool is missing. No builds claimed.';
    default:
      return STATUS_TEXT[status.kind];
  }
}

/** The builder's line of the row: what it is doing, or what turning it on or off is doing (BLD-020). */
export function builderText(enrolled: boolean | null, busy: boolean, status: BuilderStatus): string {
  if (busy) return enrolled === true ? 'Turning off…' : 'Enrolling…';
  return enrolled === true ? statusText(status) : 'Off.';
}
