import type { ChannelKind, NotificationChannel } from '$lib/notifications/api';

/** What the hub holds for one check (NOT-003, NOT-005). */
export interface StoredChannels {
  channelIds: readonly string[];
  resendEveryBeats: number;
}

/** What the section edits: the chosen channel IDs and the resend interval as typed. */
export interface ChannelValues {
  channelIds: string[];
  resend: string;
}

export const RESEND_SENTENCE = 'Whole number, 0 or more.';

export const KIND_LABELS: Record<ChannelKind, string> = {
  WEBHOOK: 'Webhook',
  NTFY: 'ntfy',
  EMAIL: 'Email',
  APPRISE: 'Apprise',
};

/** NOT-003: a new check starts with the channels the operator marked "on by default". */
export function defaultChannelIds(channels: readonly Pick<NotificationChannel, 'id' | 'onByDefault'>[]): string[] {
  return channels.filter((channel) => channel.onByDefault).map((channel) => channel.id);
}

export function channelValues(stored: StoredChannels): ChannelValues {
  return { channelIds: [...stored.channelIds], resend: String(stored.resendEveryBeats) };
}

/** NOT-005: the resend interval is a count of Down heartbeats; 0 never resends. The hub holds the upper bound. */
export function resendProblem(values: ChannelValues): string | null {
  return /^\d{1,6}$/.test(values.resend.trim()) ? null : RESEND_SENTENCE;
}

export interface CheckNotificationsInput {
  channelIds: string[];
  resendEveryBeats: number;
}

export function notificationsInput(values: ChannelValues): CheckNotificationsInput {
  return { channelIds: [...values.channelIds].toSorted(), resendEveryBeats: Number(values.resend.trim()) };
}

/** Whether `values` differ from what the hub holds, so an untouched section sends nothing. */
export function differs(stored: StoredChannels, values: ChannelValues): boolean {
  const input = notificationsInput(values);
  const held = [...stored.channelIds].toSorted();
  return input.resendEveryBeats !== stored.resendEveryBeats || input.channelIds.join() !== held.join();
}
