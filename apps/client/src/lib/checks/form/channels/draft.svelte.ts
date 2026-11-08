import { fieldErrorsFrom } from '$lib/devices/form/field-errors';
import type { NotificationChannel } from '$lib/notifications/api';
import type { ChannelCalls } from './calls';
import { channelValues, defaultChannelIds, differs, notificationsInput, RESEND_SENTENCE, resendProblem, type ChannelValues, type StoredChannels } from './channels';

export const LOAD_SENTENCE = 'Channels did not load.';
export const SAVE_SENTENCE = 'Channels were not saved.';
export const PARTIAL_SENTENCE = 'Channels saved; check was not.';

/** `loading` and `failed` keep the controls disabled and send nothing; `absent`: no hub to ask. */
export type ChannelsPhase = 'ready' | 'loading' | 'failed' | 'absent';

/** What `save` did: `sent` means the hub took new values; `failed` means it did not. */
export type ChannelsSaved = 'sent' | 'skipped' | 'failed';

/**
 * The channels and resend interval of one check in the form (NOT-003, NOT-005). A new check starts from the channels
 * that are on by default; a saved one reads its own through `calls.load`. Both go through their own documents (F-3).
 */
export class ChannelsDraft {
  channels = $state<readonly NotificationChannel[]>([]);
  values = $state<ChannelValues>({ channelIds: [], resend: '0' });
  phase = $state<ChannelsPhase>('absent');
  /** The fixed sentence for the last failure or refusal; empty when there is none. */
  message = $state('');
  /** The sentence is about the resend input, which then takes focus. */
  resendInvalid = $state(false);
  /** Try again stays mounted from the first failure until a load succeeds, so its focus survives the reload. */
  retryShown = $state(false);
  readonly #checkId: string | null;
  readonly #calls: ChannelCalls | null;
  /** What the hub holds; null until read, and for a new check. */
  #stored: StoredChannels | null = null;
  #touched = false;
  /** The hub took new channels in this form, so a check that then fails is the part that was not saved. */
  #savedHere = false;

  constructor(checkId: string | null, calls: ChannelCalls | null) {
    this.#checkId = checkId;
    this.#calls = calls;
    if (calls === null) return;
    this.phase = 'loading';
    void this.load();
  }

  /** Marks the choice as the operator's, so a slow answer does not replace it. */
  touch(): void {
    this.#touched = true;
  }

  /** Reads the channels, and the check's own choice when it is saved; a failure disables only this part. */
  async load(): Promise<void> {
    const calls = this.#calls;
    if (calls === null) return;
    this.phase = 'loading';
    this.message = '';
    this.resendInvalid = false;
    try {
      const checkId = this.#checkId;
      const [channels, own] = await Promise.all([calls.channels(), checkId === null ? null : calls.load(checkId)]);
      if (channels.ok && (own === null || own.ok)) {
        this.channels = channels.value;
        if (own !== null && own.ok) {
          this.#stored = own.value;
          this.values = channelValues(own.value);
        } else if (!this.#touched) {
          this.values.channelIds = defaultChannelIds(channels.value);
        }
        this.phase = 'ready';
        this.retryShown = false;
        return;
      }
    } catch {
      // A throwing hub is the same state as one that refused: the sentence below says it.
    }
    this.phase = 'failed';
    this.retryShown = true;
    this.message = LOAD_SENTENCE;
  }

  /** NOT-005: shows the sentence for an unusable resend interval and answers whether the form may save. */
  check(): boolean {
    const problem = this.phase === 'ready' ? resendProblem(this.values) : null;
    this.message = problem ?? '';
    this.resendInvalid = problem !== null;
    return problem === null;
  }

  /** Says the channels went in but the check did not; says nothing when the channels were never saved here. */
  checkNotSaved(): void {
    if (!this.#savedHere) return;
    this.message = PARTIAL_SENTENCE;
    this.resendInvalid = false;
  }

  /** Saves the channels of a check: only when they changed (or the check is new) and only once they have loaded. */
  async save(checkId: string): Promise<ChannelsSaved> {
    if (this.#calls === null || this.phase !== 'ready') return 'skipped';
    if (this.#stored !== null && !differs(this.#stored, this.values)) return 'skipped';
    try {
      const result = await this.#calls.save(checkId, notificationsInput(this.values));
      if (result.ok) {
        this.#stored = result.value;
        this.#savedHere = true;
        this.message = '';
        return 'sent';
      }
      this.resendInvalid = fieldErrorsFrom(result.error).some((error) => error.field === 'resendEveryBeats');
      this.message = this.resendInvalid ? RESEND_SENTENCE : SAVE_SENTENCE;
    } catch {
      this.resendInvalid = false;
      this.message = SAVE_SENTENCE;
    }
    return 'failed';
  }
}
