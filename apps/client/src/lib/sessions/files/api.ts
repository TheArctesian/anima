import { graphql } from '$lib/hub/generated';
import type { OpenFilesMutation } from '$lib/hub/generated/graphql';
import type { Hub, HubResult } from '$lib/hub/hub';

const OpenFiles = graphql(`
  mutation OpenFiles($sessionId: ID!) {
    openFiles(sessionId: $sessionId) {
      sessionId
      token
      tunnelPath
      expiresAt
    }
  }
`);

/** A single-use ticket for the files tunnel of one live SSH session (K1 `SessionTicket`, K4 §9, API-009). */
export type FilesTicket = OpenFilesMutation['openFiles'];

/** The file panel's calls through a `Hub` (CQ-045). Failures stay the hub's typed K7 errors. */
export interface FilesApi {
  /**
   * A ticket for `/files` beside the live SSH session `sessionId` of this operator and client address (K4 §9). The
   * token is spent by the one connection made with it and is never kept or shown (SEC-002, LOG-007).
   */
  openFiles(sessionId: string): Promise<HubResult<FilesTicket>>;
}

export function filesApi(hub: Hub): FilesApi {
  return {
    openFiles: async (sessionId) => {
      const result = await hub.request(OpenFiles, { sessionId });
      return result.ok ? { ok: true, value: result.value.openFiles } : result;
    },
  };
}
