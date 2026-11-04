/* eslint-disable */
/** Internal type. DO NOT USE DIRECTLY. */
type Exact<T extends { [key: string]: unknown }> = { [K in keyof T]: T[K] };
/** Internal type. DO NOT USE DIRECTLY. */
export type Incremental<T> = T | { [P in keyof T]?: P extends ' $fragmentName' | '__typename' ? T[P] : never };
import type { TypedDocumentNode as DocumentNode } from '@graphql-typed-document-node/core';
export type DisplayInput = {
  dpi?: number | null | undefined;
  heightPx: number;
  timezone?: string | null | undefined;
  /** Only these values come from the client (GW-012). */
  widthPx: number;
};

export type Protocol =
  | 'SSH';

export type SessionEndReason =
  | 'AUTH_FAILED'
  | 'GATEWAY_ERROR'
  | 'HOST_KEY_MISMATCH'
  | 'HUB_SHUTDOWN'
  | 'OPERATOR'
  /** The client sent an instruction that does not decode (GW-011, R-034). */
  | 'PROTOCOL_ERROR'
  | 'REMOTE_CLOSED'
  | 'TIMEOUT';

export type StartSessionInput = {
  deviceId: string;
  display: DisplayInput;
  protocol: Protocol;
};

export type StartSessionMutationVariables = Exact<{
  input: StartSessionInput;
}>;


export type StartSessionMutation = { startSession: { sessionId: string, token: string, tunnelPath: string, expiresAt: string } };

export type EndSessionMutationVariables = Exact<{
  id: string;
}>;


export type EndSessionMutation = { endSession: { id: string, deviceId: string | null, deviceName: string, protocol: Protocol, startedAt: string, endedAt: string | null, endReason: SessionEndReason | null } };

export type TestConnectionMutationVariables = Exact<{
  deviceId: string;
}>;


export type TestConnectionMutation = { testConnection: { reachable: boolean, authenticated: boolean, hostKeyChanged: boolean, message: string | null, hostKey: { algorithm: string, fingerprintSha256: string, firstSeenAt: string } | null } };

export type AcceptHostKeyMutationVariables = Exact<{
  deviceId: string;
  fingerprintSha256: string;
}>;


export type AcceptHostKeyMutation = { acceptHostKey: { algorithm: string, fingerprintSha256: string, firstSeenAt: string } };


export const StartSessionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"StartSession"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"input"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"StartSessionInput"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"startSession"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"Variable","name":{"kind":"Name","value":"input"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"sessionId"}},{"kind":"Field","name":{"kind":"Name","value":"token"}},{"kind":"Field","name":{"kind":"Name","value":"tunnelPath"}},{"kind":"Field","name":{"kind":"Name","value":"expiresAt"}}]}}]}}]} as unknown as DocumentNode<StartSessionMutation, StartSessionMutationVariables>;
export const EndSessionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"EndSession"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"id"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"endSession"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"id"},"value":{"kind":"Variable","name":{"kind":"Name","value":"id"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"id"}},{"kind":"Field","name":{"kind":"Name","value":"deviceId"}},{"kind":"Field","name":{"kind":"Name","value":"deviceName"}},{"kind":"Field","name":{"kind":"Name","value":"protocol"}},{"kind":"Field","name":{"kind":"Name","value":"startedAt"}},{"kind":"Field","name":{"kind":"Name","value":"endedAt"}},{"kind":"Field","name":{"kind":"Name","value":"endReason"}}]}}]}}]} as unknown as DocumentNode<EndSessionMutation, EndSessionMutationVariables>;
export const TestConnectionDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"TestConnection"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"testConnection"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"input"},"value":{"kind":"ObjectValue","fields":[{"kind":"ObjectField","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}}]}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"reachable"}},{"kind":"Field","name":{"kind":"Name","value":"authenticated"}},{"kind":"Field","name":{"kind":"Name","value":"hostKey"},"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}},{"kind":"Field","name":{"kind":"Name","value":"hostKeyChanged"}},{"kind":"Field","name":{"kind":"Name","value":"message"}}]}}]}}]} as unknown as DocumentNode<TestConnectionMutation, TestConnectionMutationVariables>;
export const AcceptHostKeyDocument = {"kind":"Document","definitions":[{"kind":"OperationDefinition","operation":"mutation","name":{"kind":"Name","value":"AcceptHostKey"},"variableDefinitions":[{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"ID"}}}},{"kind":"VariableDefinition","variable":{"kind":"Variable","name":{"kind":"Name","value":"fingerprintSha256"}},"type":{"kind":"NonNullType","type":{"kind":"NamedType","name":{"kind":"Name","value":"String"}}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"acceptHostKey"},"arguments":[{"kind":"Argument","name":{"kind":"Name","value":"deviceId"},"value":{"kind":"Variable","name":{"kind":"Name","value":"deviceId"}}},{"kind":"Argument","name":{"kind":"Name","value":"fingerprintSha256"},"value":{"kind":"Variable","name":{"kind":"Name","value":"fingerprintSha256"}}}],"selectionSet":{"kind":"SelectionSet","selections":[{"kind":"Field","name":{"kind":"Name","value":"algorithm"}},{"kind":"Field","name":{"kind":"Name","value":"fingerprintSha256"}},{"kind":"Field","name":{"kind":"Name","value":"firstSeenAt"}}]}}]}}]} as unknown as DocumentNode<AcceptHostKeyMutation, AcceptHostKeyMutationVariables>;