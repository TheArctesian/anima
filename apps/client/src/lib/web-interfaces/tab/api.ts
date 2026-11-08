import { graphql } from '$lib/hub/generated';
import type { AcceptWebCertificateMutation } from '$lib/hub/generated/graphql';
import type { Hub, HubResult } from '$lib/hub/hub';

const AcceptWebCertificate = graphql(`
  mutation AcceptWebCertificate($id: ID!, $fingerprintSha256: String!) {
    acceptWebCertificate(id: $id, fingerprintSha256: $fingerprintSha256) {
      id
      pinnedFingerprintSha256
      pinnedAt
    }
  }
`);

/** A web interface's pin as the hub holds it after an accept (K1 `WebInterface`, H-4). */
export type WebCertificatePin = AcceptWebCertificateMutation['acceptWebCertificate'];

/** The web tab's calls through a `Hub` (CQ-045). Failures stay the hub's typed K7 errors. */
export interface WebTabApi {
  /**
   * Pin `fingerprintSha256` for the web interface `id` and record it in the audit log (WEB-008, WEB-009). The hub
   * answers the pin in its own form, 64 lower-case hex; accepting again replaces the pin.
   */
  acceptWebCertificate(id: string, fingerprintSha256: string): Promise<HubResult<WebCertificatePin>>;
}

export function webTabApi(hub: Hub): WebTabApi {
  return {
    acceptWebCertificate: async (id, fingerprintSha256) => {
      const result = await hub.request(AcceptWebCertificate, { id, fingerprintSha256 });
      return result.ok ? { ok: true, value: result.value.acceptWebCertificate } : result;
    },
  };
}
