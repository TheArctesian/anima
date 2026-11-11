import { graphql } from '$lib/hub/generated';
import type { VpnQuery } from '$lib/hub/generated/graphql';
import type { Hub, HubResult, Sink } from '$lib/hub/hub';

void graphql(`
  fragment VpnNodeFields on VpnNode {
    name
    address
    os
    role
    tags
    routes
    online
    lastSeen
    keyExpiry
    keyState
    path
    relayRegion
    inAnima
    device {
      id
      name
    }
  }
`);

void graphql(`
  fragment VpnPictureFields on VpnPicture {
    readAt
    fresh
    hub {
      ...VpnNodeFields
    }
    sites {
      site {
        id
        name
        subnet
      }
      router {
        ...VpnNodeFields
      }
      state
      path
      relayRegion
      hasPingCheck
    }
    nodes {
      ...VpnNodeFields
    }
  }
`);

const Vpn = graphql(`
  query Vpn {
    vpn {
      ...VpnPictureFields
    }
  }
`);

const VpnChanged = graphql(`
  subscription VpnChanged {
    vpnChanged {
      ...VpnPictureFields
    }
  }
`);

/** The VPN as the hub's own node sees it (VPN-005). */
export type VpnPicture = NonNullable<VpnQuery['vpn']>;
export type VpnNode = VpnPicture['nodes'][number];
export type SitePath = VpnPicture['sites'][number];

/** The VPN picture and its live updates through a `Hub` (CQ-045). */
export function vpnApi(hub: Hub) {
  return {
    /** The picture now; `null` when the hub has no LocalAPI proxy or has not read it yet. */
    picture: async (): Promise<HubResult<VpnPicture | null>> => {
      const result = await hub.request(Vpn, {});
      return result.ok ? { ok: true, value: result.value.vpn ?? null } : result;
    },
    /** Listen for each new picture; returns the function that stops listening. */
    watch: (sink: Sink<VpnPicture>): (() => void) =>
      hub.subscribe(
        VpnChanged,
        {},
        {
          next: (value) => {
            sink.next(value.vpnChanged);
          },
          error: (error) => {
            sink.error(error);
          },
          complete: () => {
            sink.complete();
          },
        },
      ),
  };
}
