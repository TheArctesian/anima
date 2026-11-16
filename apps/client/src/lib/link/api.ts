import { graphql } from '$lib/hub/generated';
import type { LinkDevicesQuery } from '$lib/hub/generated/graphql';
import type { Hub, HubResult } from '$lib/hub/hub';

// Own documents (WP-50 F-3): a hub that is not in the link mode fails these only, with UNAVAILABLE.
const LINK_DEVICES = graphql(`
  query LinkDevices {
    linkDevices {
      key
      fingerprint
      name
      address
      path
      relay
      expiresAt
      streams
    }
  }
`);

const CREATE_LINK_INVITE = graphql(`
  mutation CreateLinkInvite($name: String!) {
    createLinkInvite(name: $name) {
      invite
      expiresAt
    }
  }
`);

const REVOKE_LINK_DEVICE = graphql(`
  mutation RevokeLinkDevice($key: String!) {
    revokeLinkDevice(key: $key)
  }
`);

/** One device the link permits (LNK-004). */
export type LinkDevice = LinkDevicesQuery['linkDevices'][number];

/** An invite: shown once, works once, expires (LNK-003). `invite` is a secret: never logged, stored or announced (LOG-007). */
export interface LinkInvite {
  readonly invite: string;
  readonly expiresAt: string;
}

/** The link calls of the hub (FRN-009) as plain functions of a hub. */
export interface LinkApi {
  devices(): Promise<HubResult<readonly LinkDevice[]>>;
  invite(name: string): Promise<HubResult<LinkInvite>>;
  /** How many streams the revocation closed. */
  revoke(key: string): Promise<HubResult<number>>;
}

function mapValue<T, U>(result: HubResult<T>, pick: (value: T) => U): HubResult<U> {
  return result.ok ? { ok: true, value: pick(result.value) } : result;
}

export function linkApi(hub: Pick<Hub, 'request'>): LinkApi {
  return {
    devices: async () => mapValue(await hub.request(LINK_DEVICES, {}), (value) => value.linkDevices),
    invite: async (name) => mapValue(await hub.request(CREATE_LINK_INVITE, { name }), (value) => value.createLinkInvite),
    revoke: async (key) => mapValue(await hub.request(REVOKE_LINK_DEVICE, { key }), (value) => value.revokeLinkDevice),
  };
}
