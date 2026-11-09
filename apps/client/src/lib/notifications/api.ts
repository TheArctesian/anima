import { graphql } from '$lib/hub/generated';
import type {
  ChannelKind,
  NotificationChannelFieldsFragment,
  NotificationChannelInput,
  NotificationSettingsFieldsFragment,
  NotificationSettingsInput,
  NotificationTemplateInput,
} from '$lib/hub/generated/graphql';
import type { Hub, HubResult } from '$lib/hub/hub';

// Own documents (WP-50 F-3): the notification fields of K1 are @pending, so a hub without them fails these documents
// only, and only the Notifications section shows an error. No document here selects a secret: K1 has none to select
// (D-17); a webhook gives its origin, ntfy and Apprise their server, email its host.
void graphql(`
  fragment DeliveryFields on Delivery {
    id
    channelId
    kind
    at
    ok
    reason
    code
  }
`);

void graphql(`
  fragment NotificationChannelFields on NotificationChannel {
    id
    name
    kind
    onByDefault
    settings {
      __typename
      ... on WebhookSettings {
        origin
        method
        headerNames
      }
      ... on NtfySettings {
        server
        priority
        tokenStored
      }
      ... on EmailSettings {
        host
        port
        security
        username
        from
        to
        passwordStored
      }
      ... on AppriseSettings {
        server
        mode
        tags
      }
    }
    template {
      title
      body
    }
    lastDelivery {
      ...DeliveryFields
    }
  }
`);

void graphql(`
  fragment NotificationSettingsFields on NotificationSettings {
    certificateDays
    template {
      title
      body
    }
    placeholders
  }
`);

const CHANNELS = graphql(`
  query NotificationChannels {
    notificationChannels {
      ...NotificationChannelFields
    }
  }
`);

const SETTINGS = graphql(`
  query NotificationSettings {
    notificationSettings {
      ...NotificationSettingsFields
    }
  }
`);

const CREATE = graphql(`
  mutation CreateNotificationChannel($input: NotificationChannelInput!) {
    createNotificationChannel(input: $input) {
      ...NotificationChannelFields
    }
  }
`);

const UPDATE = graphql(`
  mutation UpdateNotificationChannel($id: ID!, $input: NotificationChannelInput!) {
    updateNotificationChannel(id: $id, input: $input) {
      ...NotificationChannelFields
    }
  }
`);

const DELETE = graphql(`
  mutation DeleteNotificationChannel($id: ID!) {
    deleteNotificationChannel(id: $id)
  }
`);

const SEND_TEST = graphql(`
  mutation SendTestNotification($id: ID!) {
    sendTestNotification(id: $id) {
      ...DeliveryFields
    }
  }
`);

const SET_SETTINGS = graphql(`
  mutation SetNotificationSettings($input: NotificationSettingsInput!) {
    setNotificationSettings(input: $input) {
      ...NotificationSettingsFields
    }
  }
`);

/** One channel as the hub lists it: its target as an origin or host, never a secret (D-17). */
export type NotificationChannel = NotificationChannelFieldsFragment;
export type ChannelSettings = NotificationChannel['settings'];
/** One attempt to send; `reason` and `code` are values the client turns into fixed sentences (NOT-004, CR-14.5). */
export type Delivery = NonNullable<NotificationChannel['lastDelivery']>;
export type NotificationSettings = NotificationSettingsFieldsFragment;

export type { ChannelKind, NotificationChannelInput, NotificationSettingsInput, NotificationTemplateInput };

/** The notification calls of the hub (NOT-002 to NOT-004, NOT-006, NOT-007) as plain functions of a `Hub`. */
export interface NotificationsApi {
  channels(): Promise<HubResult<readonly NotificationChannel[]>>;
  settings(): Promise<HubResult<NotificationSettings>>;
  /** The input holds secrets: the caller drops it once the call returns and never logs it (LOG-007). */
  create(input: NotificationChannelInput): Promise<HubResult<NotificationChannel>>;
  /** A secret that is `null` keeps the stored one; `""` clears an optional one (K1). */
  update(id: string, input: NotificationChannelInput): Promise<HubResult<NotificationChannel>>;
  /** The id of the deleted channel. */
  remove(id: string): Promise<HubResult<string>>;
  /** Sends at once; the hub waits at most 8 seconds for the target (NOT-004). */
  sendTest(id: string): Promise<HubResult<Delivery>>;
  setSettings(input: NotificationSettingsInput): Promise<HubResult<NotificationSettings>>;
}

function mapValue<T, U>(result: HubResult<T>, pick: (value: T) => U): HubResult<U> {
  return result.ok ? { ok: true, value: pick(result.value) } : result;
}

export function notificationsApi(hub: Hub): NotificationsApi {
  return {
    channels: async () => mapValue(await hub.request(CHANNELS, {}), (value) => value.notificationChannels),
    settings: async () => mapValue(await hub.request(SETTINGS, {}), (value) => value.notificationSettings),
    create: async (input) => mapValue(await hub.request(CREATE, { input }), (value) => value.createNotificationChannel),
    update: async (id, input) => mapValue(await hub.request(UPDATE, { id, input }), (value) => value.updateNotificationChannel),
    remove: async (id) => mapValue(await hub.request(DELETE, { id }), (value) => value.deleteNotificationChannel),
    sendTest: async (id) => mapValue(await hub.request(SEND_TEST, { id }), (value) => value.sendTestNotification),
    setSettings: async (input) => mapValue(await hub.request(SET_SETTINGS, { input }), (value) => value.setNotificationSettings),
  };
}
