import { graphql } from '$lib/hub/generated';
import type { Hub, HubError, HubResult } from '$lib/hub/hub';

// Own documents: the VPN broker mutations are Login-or-browser-token only (D-23), and the key they return is shown once (LOG-007).
const CreateVpnInvite = graphql(`
  mutation CreateVpnInvite {
    createVpnInvite {
      key
      expiresAt
      steps
    }
  }
`);

const CreateRouterKey = graphql(`
  mutation CreateRouterKey($siteId: ID!) {
    createRouterKey(siteId: $siteId) {
      key
      expiresAt
      steps
    }
  }
`);

const RegisterVpnNode = graphql(`
  mutation RegisterVpnNode($authId: String!) {
    registerVpnNode(authId: $authId) {
      name
      address
      keyExpiry
    }
  }
`);

const RenewVpnKey = graphql(`
  mutation RenewVpnKey($name: String!) {
    renewVpnKey(name: $name) {
      name
      address
      keyExpiry
    }
  }
`);

const ApproveSiteRoute = graphql(`
  mutation ApproveSiteRoute($siteId: ID!) {
    approveSiteRoute(siteId: $siteId)
  }
`);

const NOT_APPROVED: HubError = { code: 'INTERNAL', message: 'The hub did not approve the route.' };

/**
 * A pre-auth key the hub has just made. `key` is shown once: the caller holds it only while the dialog that shows it is
 * open and never logs, stores or sends it anywhere (LOG-007). `steps` are the commands for the new computer; none holds the key.
 */
export interface VpnKey {
  readonly key: string;
  readonly expiresAt: string;
  readonly steps: readonly string[];
}

/** A node the broker registered (vpn.md 6B). */
export interface VpnRegistration {
  readonly name: string;
  readonly address: string;
  readonly keyExpiry: string | null;
}

/** The VPN enrolment, renewal and approval calls of the hub (VPN-001, VPN-003, VP-7) as plain functions of a `Hub`. */
export function enrolApi(hub: Hub) {
  return {
    /** A one-use, one-hour key for a Linux computer of the operator (VPN-003). */
    createInvite: async (): Promise<HubResult<VpnKey>> => {
      const result = await hub.request(CreateVpnInvite, {});
      return result.ok ? { ok: true, value: result.value.createVpnInvite } : result;
    },
    /** A one-use, one-hour key for the subnet router of a site (VPN-001). */
    createRouterKey: async (siteId: string): Promise<HubResult<VpnKey>> => {
      const result = await hub.request(CreateRouterKey, { siteId });
      return result.ok ? { ok: true, value: result.value.createRouterKey } : result;
    },
    /** Registers the computer that shows `authId` on its own screen (vpn.md 6B). */
    register: async (authId: string): Promise<HubResult<VpnRegistration>> => {
      const result = await hub.request(RegisterVpnNode, { authId });
      return result.ok ? { ok: true, value: result.value.registerVpnNode } : result;
    },
    /** Sets the key expiry of an untagged operator node to a staggered date 90 to 180 days away (BD-17, VP-10). */
    renew: async (name: string): Promise<HubResult<VpnRegistration>> => {
      const result = await hub.request(RenewVpnKey, { name });
      return result.ok ? { ok: true, value: result.value.renewVpnKey } : result;
    },
    /** Approves the waiting route of the site's router (R-055); the picture shows the result through `vpnChanged` (BD-16). */
    approve: async (siteId: string): Promise<HubResult<true>> => {
      const result = await hub.request(ApproveSiteRoute, { siteId });
      if (!result.ok) return result;
      return result.value.approveSiteRoute ? { ok: true, value: true } : { ok: false, error: NOT_APPROVED };
    },
  };
}

export type EnrolApi = ReturnType<typeof enrolApi>;
