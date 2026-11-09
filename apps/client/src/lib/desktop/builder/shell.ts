import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

/** A tool the builder lacks (BLD-028), as the shell names it. */
export type BuilderTool = 'git' | 'bun' | 'rust' | 'postgres' | 'tauri-packages' | 'updater-key';

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

/** A signed updater file: its name and its `.sig` text, which is public (BLD-046). */
export interface SignedFile {
  readonly file: string;
  readonly signature: string;
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
  /** Signs the updater file in the shell and checks the signature (BLD-045, BLD-046). */
  sign(job: StepJob): Promise<SignedFile>;
  /** Checks the written signature again for the job's version (BLD-046). */
  verify(job: StepJob): Promise<void>;
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
  'key-missing',
  'signed-version',
  'signing',
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
  const known = tools.filter((tool): tool is BuilderTool =>
    ['git', 'bun', 'rust', 'postgres', 'tauri-packages', 'updater-key'].includes(String(tool)),
  );
  if ((os !== 'LINUX' && os !== 'MACOS') || (arch !== 'X86_64' && arch !== 'AARCH64') || known.length !== tools.length) {
    throw new BuilderShellError('failed');
  }
  return { os, arch, missing: known };
}

function signedFileFrom(answer: unknown): SignedFile {
  if (!isRecord(answer) || typeof answer.file !== 'string' || typeof answer.signature !== 'string') {
    throw new BuilderShellError('signing');
  }
  return { file: answer.file, signature: answer.signature };
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
  sign: async (job) => signedFileFrom(await step<unknown>('builder_sign', { target: job.target, version: job.version })),
  verify: (job) => step('builder_signature_verify', { target: job.target, version: job.version }),
  cancel: () => step('builder_cancel', {}),
  onLine,
};
