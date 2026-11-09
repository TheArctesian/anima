import { createContext } from 'svelte';
import type { BuilderShell } from '$lib/desktop/builder/shell';
import type { HubResult } from '$lib/hub/hub';
import type { BuilderPlatform, BuildsApi } from './api';
import { startBuilderLoop, type BuilderStatus } from './loop';

/** Where this computer keeps its builder token: the OS key store, through the shell (BLD-023). */
export interface BuilderTokenStore {
  read(): Promise<string | null>;
  write(token: string): Promise<void>;
  clear(): Promise<void>;
}

/** Where this computer keeps its builder ID, which is not secret, so that turning building off can revoke it (BLD-025). */
export interface BuilderIdStore {
  read(): string | null;
  write(id: string): void;
  clear(): void;
}

export interface ThisBuilderPorts {
  /** `enrol` and `revoke` with the operator's session; `claim` and `report` with the builder token. */
  readonly operator: Pick<BuildsApi, 'enrol' | 'revoke'>;
  readonly builder: Pick<BuildsApi, 'claim' | 'report'>;
  readonly shell: BuilderShell;
  readonly tokens: BuilderTokenStore;
  readonly ids: BuilderIdStore;
  /** The name the hub records for this computer (BLD-021). */
  readonly computerName: () => Promise<string>;
  readonly sleep: (ms: number) => Promise<void>;
}

const PROBLEMS = {
  enrol: 'The hub did not enrol this computer as a builder.',
  store: 'The builder token could not be kept in the key store.',
  revoke: 'The hub could not revoke this builder. Revoke it in the list below.',
  read: 'The builder token could not be read from the key store.',
  tools: 'This computer could not say which platform it is.',
} as const;

/** This computer as a builder: whether it is one, and its loop (BLD-020 to BLD-029). */
export class ThisBuilder {
  /** `null` until the key store has been read. */
  enrolled = $state<boolean | null>(null);
  status = $state<BuilderStatus>({ kind: 'stopped' });
  problem = $state<string | null>(null);
  busy = $state(false);

  readonly #ports: ThisBuilderPorts;
  #token: string | null = null;
  #stop: (() => Promise<void>) | null = null;

  constructor(ports: ThisBuilderPorts) {
    this.#ports = ports;
  }

  #start(token: string): void {
    this.#token = token;
    this.#stop = startBuilderLoop({
      api: this.#ports.builder,
      shell: this.#ports.shell,
      secrets: () => (this.#token === null ? [] : [this.#token]),
      sleep: this.#ports.sleep,
      onStatus: (status) => {
        this.status = status;
      },
    });
  }

  /** Starts the loop when this computer already holds a builder token; the builder runs while the app runs (BLD-026). */
  async resume(): Promise<void> {
    try {
      const token = await this.#ports.tokens.read();
      this.enrolled = token !== null;
      if (token !== null && this.#stop === null) this.#start(token);
    } catch {
      this.enrolled = false;
      this.problem = PROBLEMS.read;
    }
  }

  /** Runs `call`; whether it worked. A failure shows `problem` (its reason may name the key store, so it is dropped: LOG-007). */
  async #attempt(problem: string, call: () => Promise<unknown>): Promise<boolean> {
    try {
      await call();
      return true;
    } catch {
      this.problem = problem;
      return false;
    }
  }

  async #platform(): Promise<BuilderPlatform | null> {
    try {
      const { os, arch } = await this.#ports.shell.checkTools();
      return { os, arch };
    } catch {
      this.problem = PROBLEMS.tools;
      return null;
    }
  }

  /** "Build releases on this computer" on: enrol with the operator's session, keep the token, start (BLD-021, BLD-023). */
  async enable(): Promise<void> {
    if (this.busy || this.enrolled === true) return;
    this.busy = true;
    this.problem = null;
    try {
      const platform = await this.#platform();
      if (platform === null) return;
      const enrolment = await this.#ports.operator.enrol(await this.#ports.computerName(), platform);
      if (!enrolment.ok) {
        this.problem = PROBLEMS.enrol;
        return;
      }
      const { builderId, token } = enrolment.value;
      if (!(await this.#attempt(PROBLEMS.store, () => this.#ports.tokens.write(token)))) {
        await this.#ports.operator.revoke(builderId);
        return;
      }
      this.#ports.ids.write(builderId);
      this.enrolled = true;
      this.#start(token);
    } finally {
      this.busy = false;
    }
  }

  /** Stops the loop and forgets the token and the builder ID on this computer. */
  async #forgetHere(): Promise<void> {
    await this.stop();
    await this.#attempt(PROBLEMS.store, () => this.#ports.tokens.clear());
    this.#ports.ids.clear();
    this.#token = null;
    this.enrolled = false;
    this.status = { kind: 'stopped' };
  }

  /** "Build releases on this computer" off: stop, revoke the token on the hub and delete it from the key store (BLD-025). */
  async disable(): Promise<void> {
    if (this.busy) return;
    this.busy = true;
    this.problem = null;
    try {
      await this.stop();
      const id = this.#ports.ids.read();
      const revoked: HubResult<string> | null = id === null ? null : await this.#ports.operator.revoke(id);
      if (revoked !== null && !revoked.ok && revoked.error.code !== 'NOT_FOUND') this.problem = PROBLEMS.revoke;
      await this.#forgetHere();
    } finally {
      this.busy = false;
    }
  }

  /** After the operator revoked builder `id` in the list: when it is this computer, forget its token too (BLD-025). */
  async revoked(id: string): Promise<void> {
    if (this.#ports.ids.read() === id) await this.#forgetHere();
  }

  /** Stops the loop, for example when the app closes; the token stays. */
  async stop(): Promise<void> {
    const stop = this.#stop;
    this.#stop = null;
    if (stop !== null) await stop();
  }
}

const [getThisBuilder, setThisBuilder] = createContext<ThisBuilder>();

/** Make `builder` available to every component below; call once in the root layout. */
export function provideThisBuilder(builder: ThisBuilder): void {
  setThisBuilder(builder);
}

/** The builder set by `provideThisBuilder`; throws when there is none. */
export function useThisBuilder(): ThisBuilder {
  return getThisBuilder();
}
