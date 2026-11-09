// Test support: a fake desktop shell and a fake builds API that record what the builder loop asks of them.
import { BuilderShellError, type BuilderShell, type BuilderShellCode, type BuilderTool, type ToolCheck } from '$lib/desktop/builder/shell';
import type { HubError, HubResult } from '$lib/hub/hub';
import type { BuildJobState, BuildsApi, ClaimedJob, JobReport } from '../api';

export const BUILDER_TOKEN = 'anb_builder-token-0123456789';

export const JOB: ClaimedJob = { id: 'job-1', target: 'LINUX_X86_64', version: '0.1.12', commit: 'a'.repeat(40) };

export function ok<T>(value: T): HubResult<T> {
  return { ok: true, value };
}

export function refused(code: HubError['code']): HubResult<never> {
  return { ok: false, error: { code, message: 'hub text' } as HubError };
}

export type StepCall = 'checkout' | 'gates' | 'bundle' | 'sign' | 'verify';

/** What one step does when it runs: print lines, wait, then end or fail. */
export interface StepScript {
  readonly lines?: readonly string[];
  readonly ms?: number;
  readonly fails?: BuilderShellCode;
}

/** A shell whose steps follow `scripts`; time passes through the injected `sleep`, so fake timers drive it. */
export class FakeShell implements BuilderShell {
  calls: StepCall[] = [];
  cancels = 0;
  tools: ToolCheck = { os: 'LINUX', arch: 'X86_64', missing: [], memoryGuarded: true };
  toolsFail: BuilderShellCode | null = null;
  scripts: Partial<Record<StepCall, StepScript>> = {};
  #listeners = new Set<(line: string) => void>();
  #cancel: (() => void) | null = null;
  readonly #sleep: (ms: number) => Promise<void>;

  constructor(sleep: (ms: number) => Promise<void>) {
    this.#sleep = sleep;
  }

  checkTools(): Promise<ToolCheck> {
    return this.toolsFail === null ? Promise.resolve(this.tools) : Promise.reject(new BuilderShellError(this.toolsFail));
  }

  missing(tools: readonly BuilderTool[]): void {
    this.tools = { ...this.tools, missing: tools };
  }

  async #run(name: StepCall): Promise<void> {
    this.calls.push(name);
    const script = this.scripts[name] ?? {};
    for (const line of script.lines ?? []) this.#listeners.forEach((listener) => listener(line));
    const cancelled = new Promise<'cancelled'>((resolve) => (this.#cancel = () => resolve('cancelled')));
    const outcome = await Promise.race([this.#sleep(script.ms ?? 0).then(() => 'done' as const), cancelled]);
    this.#cancel = null;
    if (outcome === 'cancelled') throw new BuilderShellError('cancelled');
    if (script.fails !== undefined) throw new BuilderShellError(script.fails);
  }

  checkout = () => this.#run('checkout');
  runGates = () => this.#run('gates');
  bundle = async () => {
    await this.#run('bundle');
    return 'Anima_0.1.12_amd64.AppImage';
  };
  sign = async () => {
    await this.#run('sign');
    return { file: 'Anima_0.1.12_amd64.AppImage', signature: 'c2lnbmF0dXJl' };
  };
  verify = () => this.#run('verify');

  cancel(): Promise<void> {
    this.cancels += 1;
    this.#cancel?.();
    return Promise.resolve();
  }

  onLine(listener: (line: string) => void): Promise<() => void> {
    this.#listeners.add(listener);
    return Promise.resolve(() => this.#listeners.delete(listener));
  }
}

export interface SentReport extends JobReport {
  readonly id: string;
  readonly at: number;
}

/** A builds API for the builder token: claims hand out `jobs` in turn, reports are recorded with the fake time. */
export class FakeBuildsApi implements Pick<BuildsApi, 'claim' | 'report'> {
  jobs: ClaimedJob[] = [];
  claims = 0;
  claimAnswer: HubResult<ClaimedJob | null> | null = null;
  reports: SentReport[] = [];
  reportAnswer: (report: JobReport) => HubResult<BuildJobState> = (report) => ok(report.state ?? 'BUILDING');

  claim(): Promise<HubResult<ClaimedJob | null>> {
    this.claims += 1;
    return Promise.resolve(this.claimAnswer ?? ok(this.jobs.shift() ?? null));
  }

  report(id: string, report: JobReport): Promise<HubResult<BuildJobState>> {
    const answer = this.reportAnswer(report);
    if (answer.ok) this.reports.push({ ...report, logLines: [...report.logLines], id, at: Date.now() });
    return Promise.resolve(answer);
  }

  /** Every line the hub accepted, in order. */
  lines(): string[] {
    return this.reports.flatMap((report) => report.logLines);
  }
}

export function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
