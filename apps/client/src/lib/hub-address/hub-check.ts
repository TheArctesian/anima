import type { HubResult } from '$lib/hub/hub';

/** Fixed sentences, short enough to stay on one line at 390 px; the hub's own text is never shown (NET-004, LOG-007). */
export const CHECKING = 'Testing the hub…';
export const HUB_ANSWERED = 'The hub answered.';
export const HUB_SILENT = 'The hub did not answer.';
export const HUB_ODD = 'The hub sent an odd answer.';

/**
 * The sentence for a connection test. Any answer from the hub, including a refusal because nobody is signed in, shows
 * that the address reaches a hub; only silence and unreadable answers are failures (CLI-002).
 */
export function connectionSentence(answer: HubResult<unknown>): string {
  if (answer.ok) return HUB_ANSWERED;
  switch (answer.error.code) {
    case 'unreachable':
      return HUB_SILENT;
    case 'malformed':
      return HUB_ODD;
    default:
      return HUB_ANSWERED;
  }
}
