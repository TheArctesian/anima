import type { PolicyCalls } from './calls';
import { DEFAULT_POLICY, limitProblem, policyValues, updatePolicyInput, type PolicyValues } from './policy';

export const LOAD_SENTENCE = 'Sessions did not load.';
export const PARTIAL_SENTENCE = 'Sessions saved; the device was not.';
export const SAVE_SENTENCE = 'Sessions were not saved.';

/** `loading` and `failed` keep the fields disabled and send nothing; `absent`: an edit with no hub to ask. */
type PolicyPhase = 'ready' | 'loading' | 'failed' | 'absent';

/**
 * The session limit of one device in the form (RA-036). A new device starts from the
 * defaults; a saved one reads its own values through `calls.load` and saves through `calls.save` (F-3).
 */
export class PolicyDraft {
  values = $state<PolicyValues>({ ...DEFAULT_POLICY });
  phase = $state<PolicyPhase>('ready');
  /** The fixed sentence for the last failure or refusal; empty when there is none. */
  message = $state('');
  /** The partial-save sentence, shown outside the Sessions disclosure so it is seen and announced when that is closed. */
  notice = $state('');
  /** True from a policy change the hub took until the device itself is saved. */
  #savedPolicy = false;
  /** True from a Try again until the load succeeds, so the button stays mounted and keeps focus meanwhile. */
  retrying = $state(false);
  readonly #deviceId: string | null;
  readonly #calls: PolicyCalls | null;
  /** What the hub holds; null until read, and for a new device. */
  #stored: PolicyValues | null = null;

  constructor(deviceId: string | null, calls: PolicyCalls | null) {
    this.#deviceId = deviceId;
    this.#calls = calls;
    if (deviceId === null) return;
    this.phase = calls === null ? 'absent' : 'loading';
    if (calls !== null) void this.load();
  }

  get creating(): boolean {
    return this.#deviceId === null;
  }

  /** Reads the stored values; a failure leaves the other fields alone and the policy fields disabled. Answers whether they are read. */
  async load(): Promise<boolean> {
    if (this.#deviceId === null || this.#calls === null) return false;
    this.retrying = this.retrying || this.phase === 'failed';
    this.phase = 'loading';
    this.message = '';
    this.#stored = null;
    try {
      const result = await this.#calls.load(this.#deviceId);
      if (result.ok) {
        this.#stored = policyValues(result.value);
        this.values = { ...this.#stored };
        this.phase = 'ready';
        this.retrying = false;
        return true;
      }
    } catch {
      // A throwing hub is the same state as one that refused: the sentence below says it.
    }
    this.phase = 'failed';
    this.message = LOAD_SENTENCE;
    return false;
  }

  /** Says the policy went in but the device did not, so the operator knows what a second Save will and will not send. */
  deviceNotSaved(): void {
    if (this.#savedPolicy) this.notice = PARTIAL_SENTENCE;
  }

  /** The device is saved too: nothing is left to warn about. */
  deviceSaved(): void {
    this.#savedPolicy = false;
    this.notice = '';
  }

  /** RA-036: shows the sentence for an unusable limit and answers whether the form may save. */
  check(): boolean {
    const problem = this.phase === 'ready' ? limitProblem(this.values, this.creating) : null;
    this.message = problem ?? '';
    this.notice = '';
    return problem === null;
  }

  /** Whether the values differ from what the hub holds; false until they are read. */
  get changed(): boolean {
    return this.#stored !== null && Object.keys(updatePolicyInput(this.#stored, this.values)).length > 0;
  }

  /**
   * Saves the changed `values` of a saved device, by default those shown; answers false, with the sentence, when the
   * hub did not take them. The Settings tab passes the values as they were when the change was made (D17).
   */
  async save(values: PolicyValues = this.values): Promise<boolean> {
    if (this.#deviceId === null || this.#calls === null || this.#stored === null) return true;
    const input = updatePolicyInput(this.#stored, values);
    if (Object.keys(input).length === 0) return true;
    try {
      const result = await this.#calls.save(this.#deviceId, input);
      if (result.ok) {
        this.#stored = policyValues(result.value);
        this.#savedPolicy = true;
        this.message = '';
        return true;
      }
    } catch {
      // As in `load`.
    }
    this.message = SAVE_SENTENCE;
    return false;
  }
}
