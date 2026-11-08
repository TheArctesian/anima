import type { PolicyCalls } from './calls';
import { DEFAULT_POLICY, limitProblem, policyValues, updatePolicyInput, type PolicyValues } from './policy';

export const LOAD_SENTENCE = 'The session settings could not be loaded. The other fields can still be saved.';
export const PARTIAL_SENTENCE = 'Sessions saved; the device was not.';
export const SAVE_SENTENCE = 'The session settings could not be saved. Nothing else was saved.';

/** `loading` and `failed` keep the fields disabled and send nothing; `absent`: an edit with no hub to ask. */
export type PolicyPhase = 'ready' | 'loading' | 'failed' | 'absent';

/**
 * The session limit and recording setting of one device in the form (RA-036, RA-040). A new device starts from the
 * defaults; a saved one reads its own values through `calls.load` and saves through `calls.save` (F-3).
 */
export class PolicyDraft {
  values = $state<PolicyValues>({ ...DEFAULT_POLICY });
  phase = $state<PolicyPhase>('ready');
  /** The fixed sentence for the last failure or refusal; empty when there is none. */
  message = $state('');
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

  /** Reads the stored values; a failure leaves the other fields alone and the policy fields disabled. */
  async load(): Promise<void> {
    if (this.#deviceId === null || this.#calls === null) return;
    this.phase = 'loading';
    this.message = '';
    this.#stored = null;
    try {
      const result = await this.#calls.load(this.#deviceId);
      if (result.ok) {
        this.#stored = policyValues(result.value);
        this.values = { ...this.#stored };
        this.phase = 'ready';
        return;
      }
    } catch {
      // A throwing hub is the same state as one that refused: the sentence below says it.
    }
    this.phase = 'failed';
    this.message = LOAD_SENTENCE;
  }

  /** Says the policy went in but the device did not, so the operator knows what a second Save will and will not send. */
  deviceNotSaved(): void {
    this.message = PARTIAL_SENTENCE;
  }

  /** RA-036: shows the sentence for an unusable limit and answers whether the form may save. */
  check(): boolean {
    const problem = this.phase === 'ready' ? limitProblem(this.values, this.creating) : null;
    this.message = problem ?? '';
    return problem === null;
  }

  /** Saves the changed values of a saved device; answers false, with the sentence, when the hub did not take them. */
  async save(): Promise<boolean> {
    if (this.#deviceId === null || this.#calls === null || this.#stored === null) return true;
    const input = updatePolicyInput(this.#stored, this.values);
    if (Object.keys(input).length === 0) return true;
    try {
      const result = await this.#calls.save(this.#deviceId, input);
      if (result.ok) {
        this.#stored = policyValues(result.value);
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
