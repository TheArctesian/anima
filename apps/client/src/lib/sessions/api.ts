import { graphql } from '$lib/hub/generated';
import type {
  AcceptHostKeyMutation,
  AcceptServerCertificateMutation,
  ConnectionTestInput,
  DeviceSessionsQuery,
  DeviceTransferQuery,
  EndSessionMutation,
  Protocol,
  SetRdpCredentialMutationVariables,
  StartSessionInput,
  StartSessionMutation,
  TestConnectionMutation,
  TestRemoteConnectionMutation,
} from '$lib/hub/generated/graphql';
import type { Hub, HubError, HubResult } from '$lib/hub/hub';

const StartSession = graphql(`
  mutation StartSession($input: StartSessionInput!) {
    startSession(input: $input) {
      sessionId
      token
      tunnelPath
      expiresAt
      wakeSent
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
  mutation TestConnection($input: ConnectionTestInput!) {
    testConnection(input: $input) {
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

const TestRemoteConnection = graphql(`
  mutation TestRemoteConnection($input: ConnectionTestInput!) {
    testConnection(input: $input) {
      reachable
      authenticated
      hostKey {
        algorithm
        fingerprintSha256
        firstSeenAt
      }
      hostKeyChanged
      message
      serverCertificate {
        fingerprintSha256
        subject
        notAfter
        firstSeenAt
      }
      certificateChanged
      certificateMatched
    }
  }
`);

const AcceptServerCertificate = graphql(`
  mutation AcceptServerCertificate($deviceId: ID!, $fingerprintSha256: String!) {
    acceptServerCertificate(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {
      fingerprintSha256
      subject
      notAfter
      firstSeenAt
    }
  }
`);

const DeviceTransfer = graphql(`
  query DeviceTransfer($id: ID!) {
    device(id: $id) {
      transfer {
        copy
        paste
      }
    }
  }
`);

const SetRdpCredential = graphql(`
  mutation SetRdpCredential($deviceId: ID!, $credential: PasswordCredentialInput!) {
    setRdpCredential(deviceId: $deviceId, credential: $credential) {
      credentialStored
    }
  }
`);

const SetVncCredential = graphql(`
  mutation SetVncCredential($deviceId: ID!, $credential: PasswordCredentialInput!) {
    setVncCredential(deviceId: $deviceId, credential: $credential) {
      credentialStored
    }
  }
`);

const DeviceSessions = graphql(`
  query DeviceSessions($deviceId: ID!, $first: Int, $after: String) {
    sessions(deviceId: $deviceId, first: $first, after: $after) {
      pageInfo {
        hasNextPage
        endCursor
      }
      edges {
        node {
          id
          protocol
          operatorName
          startedAt
          endedAt
          endReason
        }
      }
    }
  }
`);

/** A single-use ticket for one tunnel (API-009, K4 §1). Pass it to `openTunnel` at once and drop it. */
export type SessionTicket = StartSessionMutation['startSession'];

/** A session as `endSession` returns it; `endReason` is what the hub recorded (K4 §5). */
export type EndedSession = EndSessionMutation['endSession'];

/** The display a session asks for (GW-012): pixels, DPI and an IANA time zone. */
export type SessionDisplay = StartSessionInput['display'];

/** The protocols a Guacamole session tab serves (K4 §1); SSH has the terminal tunnel. */
export type RemoteProtocol = Exclude<Protocol, 'SSH'>;

/** An RDP server's certificate as the hub read or accepted it (K4 §3). */
export type ServerCertificate = AcceptServerCertificateMutation['acceptServerCertificate'];

export type { Protocol };

/**
 * What a connection test is given: a saved device's ID, or unsaved host, settings and credential (DEV-013); with
 * `protocol: RDP` the hub also reads the server certificate (K4 §3).
 */
export type { ConnectionTestInput };

/**
 * The result of a connection test (SEC-006, SEC-007). `serverCertificate` and `certificateChanged` are present
 * when the input names a K1 field (`protocol`, `rdp`, `vnc` or `password`); an SSH test without one is sent as in
 * Phase 1, so a hub without K1 still answers it.
 */
export type ConnectionTest = TestConnectionMutation['testConnection'] &
  Partial<Pick<TestRemoteConnectionMutation['testConnection'], 'serverCertificate' | 'certificateChanged' | 'certificateMatched'>>;

/** The device's copy and paste flags a session obeys (RA-033, K4 §8.7). */
export type SessionTransfer = NonNullable<DeviceTransferQuery['device']>['transfer'];

/** What the hub says after storing a password (K1 `RdpSettings`, `VncSettings`). */
export interface StoredCredential {
  readonly credentialStored: boolean;
}

/** A host key the hub has accepted. */
export type AcceptedHostKey = AcceptHostKeyMutation['acceptHostKey'];

/** One page of a device's session history, as the hub answers it (Section 6.7). */
export type SessionPage = DeviceSessionsQuery['sessions'];

/** The session calls through a `Hub` (CQ-045). Failures stay the hub's typed K7 errors. */
export interface SessionsApi {
  /**
   * Start a `protocol` session on `deviceId`; `CONFLICT` while no host key (SSH, K4 §6) or, for RDP, no server
   * certificate is accepted (K4 §1).
   */
  startSession(deviceId: string, protocol: Protocol, display: SessionDisplay): Promise<HubResult<SessionTicket>>;
  /** End the session, or return an ended one unchanged with its recorded `endReason` (D-4). */
  endSession(id: string): Promise<HubResult<EndedSession>>;
  /** Test a saved device (`{ deviceId }`) or unsaved form values (DEV-013); the hub refuses both at once. */
  testConnection(input: ConnectionTestInput): Promise<HubResult<ConnectionTest>>;
  /** Accept exactly `fingerprintSha256`; `CONFLICT` when it is not the key the hub saw. */
  acceptHostKey(deviceId: string, fingerprintSha256: string): Promise<HubResult<AcceptedHostKey>>;
  /** Accept exactly this RDP certificate fingerprint; `CONFLICT` when it is not the one the hub saw (K4 §3). */
  acceptServerCertificate(deviceId: string, fingerprintSha256: string): Promise<HubResult<ServerCertificate>>;
  /** The device's copy and paste flags (RA-033); `NOT_FOUND` when the device is gone. */
  transferPolicy(deviceId: string): Promise<HubResult<SessionTransfer>>;
  /**
   * Store `password` as the device's RDP or VNC credential (GW-015). A failure keeps the hub's code but has fixed
   * text, so the password can never reach the operator's screen or a log through an error (LOG-007).
   */
  setCredential(protocol: RemoteProtocol, deviceId: string, password: string): Promise<HubResult<StoredCredential>>;
  /** One page of the device's sessions; pass the previous `endCursor` as `after` for older ones (Section 6.7). */
  history(deviceId: string, first: number, after: string | null): Promise<HubResult<SessionPage>>;
}

const DEVICE_GONE: HubError = { code: 'NOT_FOUND', message: 'The device no longer exists.' };
const NOT_STORED = 'The hub did not store the password.';
const NOT_TESTED = 'The hub could not run the connection test.';

/** `error` with `text` in place of the hub's text, which might quote a password that was sent (LOG-007). */
function withoutText(error: HubError, text: string): HubError {
  // `unreachable` and `malformed` already carry the client's own text.
  if (error.code === 'unreachable' || error.code === 'malformed') return error;
  if (error.code !== 'VALIDATION') return { code: error.code, message: text };
  return { code: error.code, message: text, fields: error.fields.map(({ field }) => ({ field, message: text })) };
}

async function storeCredential(hub: Hub, protocol: RemoteProtocol, variables: SetRdpCredentialMutationVariables): Promise<HubResult<StoredCredential>> {
  if (protocol === 'RDP') {
    const result = await hub.request(SetRdpCredential, variables);
    return result.ok ? { ok: true, value: result.value.setRdpCredential } : result;
  }
  const result = await hub.request(SetVncCredential, variables);
  return result.ok ? { ok: true, value: result.value.setVncCredential } : result;
}

/** Whether `input` carries a secret (a password, key or passphrase) the hub's error text might quote back. */
function carriesSecret(input: ConnectionTestInput): boolean {
  return [input.password, input.credential].some((field) => field !== undefined && field !== null);
}

/** Whether `input` uses a K1 field, so only a hub with K1 can answer it. */
function namesK1(input: ConnectionTestInput): boolean {
  return [input.protocol, input.rdp, input.vnc, input.password].some((field) => field !== undefined && field !== null);
}

export function sessionsApi(hub: Hub): SessionsApi {
  return {
    startSession: async (deviceId, protocol, display) => {
      const result = await hub.request(StartSession, { input: { deviceId, protocol, display } });
      return result.ok ? { ok: true, value: result.value.startSession } : result;
    },
    endSession: async (id) => {
      const result = await hub.request(EndSession, { id });
      return result.ok ? { ok: true, value: result.value.endSession } : result;
    },
    testConnection: async (input) => {
      const result = namesK1(input)
        ? await hub.request(TestRemoteConnection, { input })
        : await hub.request(TestConnection, { input });
      if (result.ok) return { ok: true, value: result.value.testConnection };
      return carriesSecret(input) ? { ok: false, error: withoutText(result.error, NOT_TESTED) } : result;
    },
    acceptHostKey: async (deviceId, fingerprintSha256) => {
      const result = await hub.request(AcceptHostKey, { deviceId, fingerprintSha256 });
      return result.ok ? { ok: true, value: result.value.acceptHostKey } : result;
    },
    acceptServerCertificate: async (deviceId, fingerprintSha256) => {
      const result = await hub.request(AcceptServerCertificate, { deviceId, fingerprintSha256 });
      return result.ok ? { ok: true, value: result.value.acceptServerCertificate } : result;
    },
    transferPolicy: async (id) => {
      const result = await hub.request(DeviceTransfer, { id });
      if (!result.ok) return result;
      return result.value.device === null ? { ok: false, error: DEVICE_GONE } : { ok: true, value: result.value.device.transfer };
    },
    setCredential: async (protocol, deviceId, password) => {
      const result = await storeCredential(hub, protocol, { deviceId, credential: { password } });
      return result.ok ? result : { ok: false, error: withoutText(result.error, NOT_STORED) };
    },
    history: async (deviceId, first, after) => {
      const result = await hub.request(DeviceSessions, { deviceId, first, after });
      return result.ok ? { ok: true, value: result.value.sessions } : result;
    },
  };
}
