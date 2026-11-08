// The channel list as values: the row the skeleton is drawn from, a delivery put on its channel, and where focus
// goes after a delete (NOT-002, NOT-004, NAV-005).
import type { Delivery, NotificationChannel } from '../api';

/**
 * A channel of typical length that never reaches the screen as text: the loading skeleton and the space the list
 * keeps are drawn from it, so they have a real row's geometry at every width (MOT-007).
 */
export const PLACEHOLDER_CHANNEL: NotificationChannel = {
  id: 'placeholder',
  name: 'placeholder-channel',
  kind: 'WEBHOOK',
  onByDefault: false,
  settings: { __typename: 'WebhookSettings', origin: 'https://hooks.example.net', method: 'POST', headerNames: [] },
  template: null,
  lastDelivery: {
    id: 'placeholder',
    channelId: 'placeholder',
    kind: 'TEST',
    at: '2099-12-31T00:00:00Z',
    ok: false,
    reason: 'TIMED_OUT',
    code: null,
  },
};

/** The list with `delivery` as the last delivery of its channel. */
export function withDelivery(channels: readonly NotificationChannel[], id: string, delivery: Delivery): NotificationChannel[] {
  return channels.map((channel) => (channel.id === id ? { ...channel, lastDelivery: delivery } : channel));
}

/** The list with `saved` in place of the channel it updates, or added at the end. */
export function withSaved(channels: readonly NotificationChannel[], saved: NotificationChannel): NotificationChannel[] {
  const known = channels.some((channel) => channel.id === saved.id);
  return known ? channels.map((channel) => (channel.id === saved.id ? saved : channel)) : [...channels, saved];
}

/**
 * Where focus goes after the channel at `index` was deleted: the id of the channel that took its place, else the one
 * before it, else `null` for "Add channel". `channels` is the list without the deleted one.
 */
export function focusAfterDelete(channels: readonly Pick<NotificationChannel, 'id'>[], index: number): string | null {
  return (channels[index] ?? channels[index - 1])?.id ?? null;
}
