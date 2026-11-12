import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

const TOOLS = ['git', 'bun', 'rust', 'cargo-nextest', 'postgres', 'tauri-packages', 'updater-public-key'] as const;

/** A tool the builder lacks (BLD-028), as the shell names it. The builder holds no updater key: the hub signs (R-116). */
export type BuilderTool = (typeof TOOLS)[number];

/** This computer's platform, as K1 `BuilderPlatformInput` names it, and the tools it lacks. */
export interface ToolCheck {
  readonly os: 'LINUX' | 'MACOS';
  readonly arch: 'X86_64' | 'AARCH64';
  readonly missing: readonly BuilderTool[];
}

/** The job fields a step needs, as the hub sent them; the shell checks each before any process starts (BLD-027). */
export interface StepJob {
  readonly commit: string;
  readonly target: string;
  readonly version: string;
}

/** One file of the job's release: its name, lower-case hex SHA-256 and size in bytes (BLD-063). */
export interface ReleaseFile {
  readonly name: string;
  readonly sha256: string;
  readonly size: number;
}

/**
 * The files of the last bundle, `SHA256SUMS` last as the shell wrote it, and the name of its updater file (BLD-063,
 * UPD-010). Unsigned: the hub signs the updater file it checked (R-116).
 */
export interface ReleaseFiles {
  readonly files: readonly ReleaseFile[];
  readonly updater: string;
}

/** A presigned `PUT` from the hub. The URL is a credential: it goes to the shell and nowhere else (BLD-049). */
export interface PresignedUpload {
  readonly name: string;
  readonly url: string;
  readonly headers: readonly { readonly name: string; readonly value: string }[];
}

/**
 * The fixed build steps of the desktop shell (BLD-026, BLD-027). No call names a program, a path or an argument list.
 * Each step rejects with a `BuilderShellError`.
 */
export interface BuilderShell {
  checkTools(): Promise<ToolCheck>;
  /** Fetches and checks out `commit` with a clean tree; `not-on-main` when it is not on main (BLD-040, BLD-041). */
  checkout(commit: string): Promise<void>;
  /** Runs the gates at `commit` (BLD-042). */
  runGates(commit: string): Promise<void>;
  /** Compiles and bundles the job; resolves with the updater file's name (BLD-043). */
  bundle(job: StepJob): Promise<string>;
  /** The files of the job's bundle with their sizes and SHA-256, read by the shell (BLD-063, D-22). */
  releaseFiles(job: StepJob): Promise<ReleaseFiles>;
  /**
   * Sends one file of the job's bundle, or the `SHA256SUMS` the shell writes, to its presigned URL (BLD-060, BLD-063).
   * The shell checks the URL is for that file of this job, checks its headers and follows no redirect.
   */
  upload(job: StepJob, upload: PresignedUpload): Promise<void>;
  /** Stops the running step and every process it started. */
  cancel(): Promise<void>;
  /** Hears each output line of the running step; resolves with the function that stops listening. */
  onLine(listener: (line: string) => void): Promise<() => void>;
}

const SHELL_CODES = [
  'busy',
  'commit',
  'version',
  'target',
  'platform',
  'not-on-main',
  'dirty',
  'tool-missing',
  'step-failed',
  'cancelled',
  'bundle',
  'upload',
  'caller',
  'failed',
] as const;

/** Why a step did not finish; `not-desktop` is the client's own. */
export type BuilderShellCode = (typeof SHELL_CODES)[number] | 'not-desktop';

/** A step failure. Only the shell's code is kept; its lines went to `onLine` (LOG-007). */
export class BuilderShellError extends Error {
  constructor(readonly code: BuilderShellCode) {
    super(`The build step failed: ${code}.`);
    this.name = 'BuilderShellError';
  }
}

function codeOf(reason: unknown): BuilderShellCode {
  const code = typeof reason === 'object' && reason !== null && 'code' in reason ? reason.code : undefined;
  return SHELL_CODES.find((known) => known === code) ?? 'failed';
}

async function step<T>(command: string, args: Record<string, unknown>): Promise<T> {
  if (!isTauri()) throw new BuilderShellError('not-desktop');
  try {
    return await invoke<T>(command, args);
  } catch (reason) {
    throw new BuilderShellError(codeOf(reason));
  }
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

/** The shell's tool check, with each field checked: it crosses a process boundary. */
export function toolCheckFrom(answer: unknown): ToolCheck {
  if (!isRecord(answer)) throw new BuilderShellError('failed');
  const { os, arch, missing } = answer;
  const tools: readonly unknown[] = Array.isArray(missing) ? missing : [];
  const known = tools.filter((tool): tool is BuilderTool => TOOLS.some((name) => name === tool));
  const platform = (os === 'LINUX' || os === 'MACOS') && (arch === 'X86_64' || arch === 'AARCH64');
  if (!platform || known.length !== tools.length) throw new BuilderShellError('failed');
  return { os, arch, missing: known };
}

const SHA256 = /^[0-9a-f]{64}$/;

function releaseFileFrom(answer: unknown): ReleaseFile {
  if (!isRecord(answer)) throw new BuilderShellError('bundle');
  const { name, sha256, size } = answer;
  const sized = typeof size === 'number' && Number.isSafeInteger(size) && size > 0;
  if (typeof name !== 'string' || typeof sha256 !== 'string' || !SHA256.test(sha256) || !sized) {
    throw new BuilderShellError('bundle');
  }
  return { name, sha256, size };
}

/** The shell's release files, with each field checked: they cross a process boundary and go to the hub. */
export function releaseFilesFrom(answer: unknown): ReleaseFiles {
  if (!isRecord(answer) || !Array.isArray(answer.files) || answer.files.length === 0) throw new BuilderShellError('bundle');
  const { updater } = answer;
  if (typeof updater !== 'string' || updater === '') throw new BuilderShellError('bundle');
  return { files: answer.files.map(releaseFileFrom), updater };
}

/** The `builder-line` events of the main web view (BLD-048). */
async function onLine(listener: (line: string) => void): Promise<() => void> {
  if (!isTauri()) return () => undefined;
  return listen<unknown>('builder-line', (event) => {
    const payload = event.payload;
    if (isRecord(payload) && typeof payload.text === 'string') listener(payload.text);
  });
}

export const desktopBuilderShell: BuilderShell = {
  checkTools: async () => toolCheckFrom(await step<unknown>('builder_tools_check', {})),
  checkout: (commit) => step('builder_checkout', { commit }),
  runGates: (commit) => step('builder_gates_run', { commit }),
  bundle: (job) => step('builder_bundle', { job }),
  releaseFiles: async (job) =>
    releaseFilesFrom(await step<unknown>('builder_release_files', { target: job.target, version: job.version })),
  upload: (job, upload) =>
    step('builder_upload', {
      job: { target: job.target, version: job.version, upload: { name: upload.name, url: upload.url, headers: upload.headers } },
    }),
  cancel: () => step('builder_cancel', {}),
  onLine,
};
