// How a delivery reads in the list: when, delivered or failed, and why (NOT-004). The hub sends a reason as a value
// and, for some reasons, a status code (CR-14.5); every sentence here is the client's own. No hub or target text
// reaches the screen, and the target's reply is never part of a delivery (K1).
import type { DeliveryReason, NotificationKind } from '$lib/hub/generated/graphql';
import type { Delivery } from '../api';

const KIND_LABELS: Record<NotificationKind, string> = {
  DOWN: 'Down',
  UP: 'Up',
  STILL_DOWN: 'Still down',
  CERTIFICATE_EXPIRY: 'Certificate expiry',
  DOMAIN_EXPIRY: 'Domain expiry',
  TEST: 'Test',
};

/** Said for a failed delivery that carries no sentence. */
export const NO_REASON = 'The hub gave no reason.';
export const NEVER_SENT = 'Nothing sent yet.';
export const SENDING = 'Sending a test…';

export function notificationLabel(kind: NotificationKind): string {
  return KIND_LABELS[kind];
}

/** "Delivered" or "Failed": the word carries the state, never colour alone (UI-003). */
export function outcomeWord(delivery: Pick<Delivery, 'ok'>): string {
  return delivery.ok ? 'Delivered' : 'Failed';
}

/** Said for a reason this client does not know, for example one a newer hub added (CR-14.5). */
export const UNKNOWN_REASON = 'The delivery failed for a reason this version of Anima does not know.';

// The `Record` makes every K1 value need a sentence; lookups go through a map, so a value such as `constructor` finds nothing.
const SENTENCES: Record<DeliveryReason, string> = {
  TARGET_NOT_PERMITTED: 'The target address is not permitted.',
  BAD_ADDRESS: 'The target address is not valid.',
  SECRET_UNREADABLE: 'The hub could not open the stored secrets. Enter them again.',
  SETUP_FAILED: 'The hub could not prepare the message.',
  CONNECTION_FAILED: 'The hub could not connect to the target.',
  TLS_FAILED: "The target's TLS certificate was not accepted.",
  TIMED_OUT: 'The target did not answer in 8 seconds.',
  HTTP_STATUS: 'The target answered with an error status.',
  NO_STARTTLS: 'The mail server does not offer STARTTLS.',
  SMTP_REFUSED: 'The mail server refused the message.',
  NOT_UNDERSTOOD: 'The target did not understand the message.',
};
const REASONS = new Map<string, string>(Object.entries(SENTENCES));

/** A code is shown only as a number in the range its protocol uses: HTTP 100..599, SMTP 200..599. */
function codeText(reason: string, code: number | null): string | null {
  if (code === null || !Number.isInteger(code)) return null;
  if (reason === 'HTTP_STATUS' && code >= 100 && code <= 599) return `The target answered with HTTP status ${String(code)}.`;
  if (reason === 'SMTP_REFUSED' && code >= 200 && code <= 599) return `The mail server refused the message (reply ${String(code)}).`;
  return null;
}

/** A fixed sentence for why a delivery failed, or nothing for one that went through. */
export function reasonOf(delivery: Pick<Delivery, 'ok' | 'reason' | 'code'>): string {
  if (delivery.ok) return '';
  if (delivery.reason === null) return NO_REASON;
  // The value is checked at run time: a hub may send a member this build's types do not have.
  const reason: string = delivery.reason;
  return codeText(reason, delivery.code) ?? REASONS.get(reason) ?? UNKNOWN_REASON;
}

/** A time for the operator, for example "6 Nov 2026, 08:05"; a dash for a time the client cannot read. */
export function shownTime(iso: string, locale?: string): string {
  const at = new Date(iso);
  return Number.isNaN(at.getTime()) ? '—' : at.toLocaleString(locale, { dateStyle: 'medium', timeStyle: 'short' });
}

/** What Send test says once the hub answers with the test's delivery (NOT-004). */
export function testOutcome(delivery: Pick<Delivery, 'ok' | 'reason' | 'code'>): string {
  const reason = reasonOf(delivery);
  const head = delivery.ok ? 'Test delivered.' : 'Test failed.';
  return reason === '' ? head : `${head} ${reason}`;
}
