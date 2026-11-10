import { CheckNotifications, SetCheckNotifications } from '$lib/checks/api';
import { useHub } from '$lib/hub/context';
import type { Hub, HubResult } from '$lib/hub/hub';
import { notificationsApi, type NotificationChannel } from '$lib/notifications/api';
import type { CheckNotificationsInput, StoredChannels } from './channels';

/** The calls of the Notifications part of the check form. */
export interface ChannelCalls {
  /** Every channel the operator has made, for the checklist. */
  channels: () => Promise<HubResult<readonly NotificationChannel[]>>;
  load: (checkId: string) => Promise<HubResult<StoredChannels>>;
  save: (checkId: string, input: CheckNotificationsInput) => Promise<HubResult<StoredChannels>>;
}

const NOT_FOUND: HubResult<never> = { ok: false, error: { code: 'NOT_FOUND', message: 'No such check' } };

function stored(value: { channels: readonly { id: string }[]; resendEveryBeats: number }): StoredChannels {
  return { channelIds: value.channels.map((channel) => channel.id), resendEveryBeats: value.resendEveryBeats };
}

function channelCalls(hub: Hub): ChannelCalls {
  return {
    channels: () => notificationsApi(hub).channels(),
    load: async (checkId) => {
      const result = await hub.request(CheckNotifications, { id: checkId });
      if (!result.ok) return result;
      return result.value.check === null ? NOT_FOUND : { ok: true, value: stored(result.value.check.notifications) };
    },
    save: async (checkId, input) => {
      const result = await hub.request(SetCheckNotifications, { checkId, input });
      return result.ok ? { ok: true, value: stored(result.value.setCheckNotifications) } : result;
    },
  };
}

/** Svelte's error for a context nobody set; its message carries the name in development and the docs URL in production. */
function isMissingContext(error: unknown): boolean {
  return error instanceof Error && error.message.includes('missing_context');
}

/** The calls of the hub above the form, or `null` when there is none: the form then has no Notifications part. Call it while the component is created. */
export function contextChannelCalls(): ChannelCalls | null {
  try {
    return channelCalls(useHub());
  } catch (error) {
    if (isMissingContext(error)) return null;
    throw error;
  }
}
