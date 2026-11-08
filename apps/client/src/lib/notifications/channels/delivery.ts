// How a delivery reads in the list: when, delivered or failed, and the hub's fixed sentence (NOT-004). The target's
// reply is never part of a delivery (K1), so nothing here can show it.
import type { NotificationKind } from '$lib/hub/generated/graphql';
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

/** The hub's fixed sentence, a stand-in for a failure without one, or nothing for a delivery that went through. */
export function reasonOf(delivery: Pick<Delivery, 'ok' | 'message'>): string {
  return delivery.message ?? (delivery.ok ? '' : NO_REASON);
}

/** A time for the operator, for example "6 Nov 2026, 08:05"; a dash for a time the client cannot read. */
export function shownTime(iso: string, locale?: string): string {
  const at = new Date(iso);
  return Number.isNaN(at.getTime()) ? '—' : at.toLocaleString(locale, { dateStyle: 'medium', timeStyle: 'short' });
}

/** What Send test says once the hub answers with the test's delivery (NOT-004). */
export function testOutcome(delivery: Pick<Delivery, 'ok' | 'message'>): string {
  const reason = reasonOf(delivery);
  const head = delivery.ok ? 'Test delivered.' : 'Test failed.';
  return reason === '' ? head : `${head} ${reason}`;
}
