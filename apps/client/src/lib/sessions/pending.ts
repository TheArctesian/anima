// The K1 delta parts the RDP and VNC tabs use, typed by hand (CON-003): `contracts/schema.graphql` is still the
// Phase 1 freeze, and codegen reads only `src/lib/*/api.ts`, so nothing here is checked against a schema. The
// selections follow the K1 delta exactly. Once K1 is in `contracts/`, these become `graphql()` documents in
// `api.ts` and this file is deleted.
import type { TypedDocumentNode } from '@graphql-typed-document-node/core';
import { parse } from 'graphql';
import type { Variables } from '$lib/hub/hub';
import type {
  ConnectionTestInput,
  DisplayInput,
  SessionEndReason as FrozenEndReason,
  StartSessionMutation,
  TestConnectionMutation,
} from '$lib/hub/generated/graphql';

/** K1 `Protocol` with RDP and VNC. */
export type Protocol = 'SSH' | 'RDP' | 'VNC';

/** K1 `SessionEndReason` with the RDP certificate change (K4 §5). */
export type SessionEndReason = FrozenEndReason | 'SERVER_CERTIFICATE_MISMATCH';

/** K1 `TransferPolicy`: what may leave or enter the device in a session (RA-032, RA-033). */
export interface TransferPolicy {
  readonly upload: boolean;
  readonly download: boolean;
  readonly copy: boolean;
  readonly paste: boolean;
}

/** K1 `ServerCertificate`: an RDP server's TLS certificate as the hub read it (K4 §3). */
export interface ServerCertificate {
  readonly fingerprintSha256: string;
  readonly subject: string | null;
  readonly notAfter: string | null;
  readonly firstSeenAt: string;
}

export type RdpSecurity = 'ANY' | 'NLA' | 'NLA_EXT' | 'TLS' | 'VMCONNECT' | 'RDP';
export type RdpResizeMethod = 'DISPLAY_UPDATE' | 'RECONNECT';

/** K1 `RdpSettingsInput`; omitted fields take the hub's defaults. */
export interface RdpSettingsInput {
  readonly enabled: boolean;
  readonly port?: number | null;
  readonly username: string;
  readonly domain?: string | null;
  readonly security?: RdpSecurity | null;
  readonly ignoreCertificate?: boolean | null;
  readonly resizeMethod?: RdpResizeMethod | null;
  readonly audioOut?: boolean | null;
  readonly microphone?: boolean | null;
  readonly sharedDrive?: boolean | null;
}

/** K1 `VncSettingsInput`. */
export interface VncSettingsInput {
  readonly enabled: boolean;
  readonly port?: number | null;
  readonly username?: string | null;
}

/** K1 `PasswordCredentialInput`. */
export interface PasswordCredentialInput {
  readonly password: string;
}

/** K1 `ConnectionTestInput`: the Phase 1 fields plus the protocol, its settings and a password. */
export type RemoteConnectionTestInput = ConnectionTestInput & {
  readonly protocol?: Protocol | null;
  readonly rdp?: RdpSettingsInput | null;
  readonly vnc?: VncSettingsInput | null;
  readonly password?: PasswordCredentialInput | null;
};

export interface DeviceTransferQuery { device: { transfer: { copy: boolean; paste: boolean } } | null }
export interface DeviceTransferQueryVariables extends Variables { id: string }

/** K1 `Query.device(id) { transfer }`: the copy and paste flags a session obeys (RA-033, K4 §8.7). */
export const DeviceTransfer = parse(`
  query DeviceTransfer($id: ID!) {
    device(id: $id) {
      transfer {
        copy
        paste
      }
    }
  }
`) as TypedDocumentNode<DeviceTransferQuery, DeviceTransferQueryVariables>;

export interface CredentialVariables extends Variables { deviceId: string; credential: PasswordCredentialInput }
export interface SetRdpCredentialMutation { setRdpCredential: { credentialStored: boolean } }
export interface SetVncCredentialMutation { setVncCredential: { credentialStored: boolean } }

/** K1 `setRdpCredential`. */
export const SetRdpCredential = parse(`
  mutation SetRdpCredential($deviceId: ID!, $credential: PasswordCredentialInput!) {
    setRdpCredential(deviceId: $deviceId, credential: $credential) {
      credentialStored
    }
  }
`) as TypedDocumentNode<SetRdpCredentialMutation, CredentialVariables>;

/** K1 `setVncCredential`. */
export const SetVncCredential = parse(`
  mutation SetVncCredential($deviceId: ID!, $credential: PasswordCredentialInput!) {
    setVncCredential(deviceId: $deviceId, credential: $credential) {
      credentialStored
    }
  }
`) as TypedDocumentNode<SetVncCredentialMutation, CredentialVariables>;

export interface AcceptServerCertificateMutation { acceptServerCertificate: ServerCertificate }
export interface AcceptServerCertificateMutationVariables extends Variables { deviceId: string; fingerprintSha256: string }

/** K1 `acceptServerCertificate`: `CONFLICT` unless the fingerprint is the one the hub saw. */
export const AcceptServerCertificate = parse(`
  mutation AcceptServerCertificate($deviceId: ID!, $fingerprintSha256: String!) {
    acceptServerCertificate(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {
      fingerprintSha256
      subject
      notAfter
      firstSeenAt
    }
  }
`) as TypedDocumentNode<AcceptServerCertificateMutation, AcceptServerCertificateMutationVariables>;

export type RemoteConnectionTest = TestConnectionMutation['testConnection'] & {
  serverCertificate: ServerCertificate | null;
  certificateChanged: boolean;
};
export interface TestRemoteConnectionMutation { testConnection: RemoteConnectionTest }
export interface TestRemoteConnectionMutationVariables extends Variables { input: RemoteConnectionTestInput }

/** K1 `testConnection` with the K1 input fields and the server certificate (K4 §3). */
export const TestRemoteConnection = parse(`
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
    }
  }
`) as TypedDocumentNode<TestRemoteConnectionMutation, TestRemoteConnectionMutationVariables>;

export interface StartRemoteSessionMutationVariables extends Variables {
  input: { deviceId: string; protocol: Exclude<Protocol, 'SSH'>; display: DisplayInput };
}

/** K1 `startSession` with RDP or VNC; the answer is the Phase 1 ticket (K4 §1). */
export const StartRemoteSession = parse(`
  mutation StartRemoteSession($input: StartSessionInput!) {
    startSession(input: $input) {
      sessionId
      token
      tunnelPath
      expiresAt
    }
  }
`) as TypedDocumentNode<StartSessionMutation, StartRemoteSessionMutationVariables>;
