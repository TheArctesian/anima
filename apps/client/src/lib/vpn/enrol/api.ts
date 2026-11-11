import { graphql } from '$lib/hub/generated';
import type { Hub, HubResult } from '$lib/hub/hub';

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

/** The VPN enrolment calls of the hub (VPN-001, VPN-003, VP-7) as plain functions of a `Hub`. */
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
  };
}

export type EnrolApi = ReturnType<typeof enrolApi>;
