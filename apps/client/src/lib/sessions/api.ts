import { graphql } from '$lib/hub/generated';
import type {
  AcceptHostKeyMutation,
  EndSessionMutation,
  StartSessionInput,
  StartSessionMutation,
  TestConnectionMutation,
} from '$lib/hub/generated/graphql';
import type { Hub, HubResult } from '$lib/hub/hub';

const StartSession = graphql(`
  mutation StartSession($input: StartSessionInput!) {
    startSession(input: $input) {
      sessionId
      token
      tunnelPath
      expiresAt
    }
  }
`);

const EndSession = graphql(`
  mutation EndSession($id: ID!) {
    endSession(id: $id) {
      id
      deviceId
      deviceName
      protocol
      startedAt
      endedAt
      endReason
    }
  }
`);

const TestConnection = graphql(`
  mutation TestConnection($deviceId: ID!) {
    testConnection(input: { deviceId: $deviceId }) {
      reachable
      authenticated
      hostKey {
        algorithm
        fingerprintSha256
        firstSeenAt
      }
      hostKeyChanged
      message
    }
  }
`);

const AcceptHostKey = graphql(`
  mutation AcceptHostKey($deviceId: ID!, $fingerprintSha256: String!) {
    acceptHostKey(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {
      algorithm
      fingerprintSha256
      firstSeenAt
    }
  }
`);

/** A single-use ticket for one tunnel (API-009, K4 §1). Pass it to `openTunnel` at once and drop it. */
export type SessionTicket = StartSessionMutation['startSession'];

/** A session as `endSession` returns it; `endReason` is what the hub recorded (K4 §5). */
export type EndedSession = EndSessionMutation['endSession'];

/** The display a session asks for (GW-012): pixels, DPI and an IANA time zone. */
export type SessionDisplay = StartSessionInput['display'];

/** The connection test of a saved device (SEC-006, SEC-007). */
export type ConnectionTest = TestConnectionMutation['testConnection'];

/** A host key the hub has accepted. */
export type AcceptedHostKey = AcceptHostKeyMutation['acceptHostKey'];

/** The session calls through a `Hub` (CQ-045). Failures stay the hub's typed K7 errors. */
export interface SessionsApi {
  /** Start an SSH session on `deviceId`; `CONFLICT` while no host key is accepted (K4 §6). */
  startSession(deviceId: string, display: SessionDisplay): Promise<HubResult<SessionTicket>>;
  /** End the session, or return an ended one unchanged with its recorded `endReason` (D-4). */
  endSession(id: string): Promise<HubResult<EndedSession>>;
  testConnection(deviceId: string): Promise<HubResult<ConnectionTest>>;
  /** Accept exactly `fingerprintSha256`; `CONFLICT` when it is not the key the hub saw. */
  acceptHostKey(deviceId: string, fingerprintSha256: string): Promise<HubResult<AcceptedHostKey>>;
}

export function sessionsApi(hub: Hub): SessionsApi {
  return {
    startSession: async (deviceId, display) => {
      const result = await hub.request(StartSession, { input: { deviceId, protocol: 'SSH', display } });
      return result.ok ? { ok: true, value: result.value.startSession } : result;
    },
    endSession: async (id) => {
      const result = await hub.request(EndSession, { id });
      return result.ok ? { ok: true, value: result.value.endSession } : result;
    },
    testConnection: async (deviceId) => {
      const result = await hub.request(TestConnection, { deviceId });
      return result.ok ? { ok: true, value: result.value.testConnection } : result;
    },
    acceptHostKey: async (deviceId, fingerprintSha256) => {
      const result = await hub.request(AcceptHostKey, { deviceId, fingerprintSha256 });
      return result.ok ? { ok: true, value: result.value.acceptHostKey } : result;
    },
  };
}
