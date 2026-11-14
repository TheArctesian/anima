import type { HubResult } from '$lib/hub/hub';
import type { ConnectAttempt, GithubConnectApi } from './api';
import { isOpen } from './attempt';
import type { LinkOpening } from './browser';

/**
 * The one GitHub connection the hub holds (PRJ-039). `attempt` follows `githubConnectChanged` with no reload. The
 * one-time link lives in a local variable between the hub's answer and the browser: it is not a field, not stored,
 * not logged and not in a message (PRJ-041).
 */
export class GithubConnectStore {
  attempt = $state.raw<ConnectAttempt | null>(null);
  /** The hub dropped an attempt that the client held open (it restarted). */
  restarted = $state(false);
  /** The browser did not show the link: the page offers `attempt.pageUrl` instead. */
  blocked = $state(false);
  /** The hub's words for the last refused action. */
  refusal = $state('');
  busy = $state(false);
  #unwatch: (() => void) | null = null;

  constructor(private readonly api: GithubConnectApi) {}

  /** Reads the open attempt; null after an open one means the hub restarted. */
  async load(): Promise<void> {
    const held = this.attempt !== null && isOpen(this.attempt);
    const result = await this.api.current();
    if (!result.ok) return;
    this.restarted = result.value === null && held;
    this.attempt = result.value;
  }

  /** Reads the attempt and listens for changes (once). */
  async ensureLoaded(): Promise<void> {
    await this.load();
    this.#unwatch ??= this.api.watch({
      next: (attempt) => {
        this.restarted = false;
        this.attempt = attempt;
      },
      error: () => undefined,
      complete: () => undefined,
    });
  }

  stop(): void {
    this.#unwatch?.();
    this.#unwatch = null;
  }

  /** Press 1: starts an attempt for the personal account and opens its link. */
  async start(opening: LinkOpening): Promise<void> {
    this.#begin();
    const result = await this.api.start();
    if (!result.ok) return this.#refused(result, opening);
    this.restarted = false;
    this.attempt = result.value;
    await this.#open(result.value.id, opening);
  }

  /** Asks for a new one-time link for the open attempt and opens it. */
  async continueInBrowser(opening: LinkOpening): Promise<void> {
    if (this.attempt === null) return opening.close();
    this.#begin();
    await this.#open(this.attempt.id, opening);
  }

  async cancel(): Promise<void> {
    if (this.attempt === null) return;
    this.#begin();
    const result = await this.api.cancel(this.attempt.id);
    if (result.ok) this.attempt = result.value;
    else this.refusal = result.error.message;
    this.busy = false;
  }

  async #open(id: string, opening: LinkOpening): Promise<void> {
    const link = await this.api.link(id);
    if (!link.ok) return this.#refused(link, opening);
    this.blocked = !(await opening.open(link.value));
    this.busy = false;
  }

  #begin(): void {
    this.refusal = '';
    this.blocked = false;
    this.busy = true;
  }

  #refused(result: Extract<HubResult<unknown>, { ok: false }>, opening: LinkOpening): void {
    opening.close();
    this.refusal = result.error.message;
    this.busy = false;
  }
}
