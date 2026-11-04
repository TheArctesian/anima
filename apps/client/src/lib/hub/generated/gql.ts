/* eslint-disable */
import * as types from './graphql';
import type { TypedDocumentNode as DocumentNode } from '@graphql-typed-document-node/core';

/**
 * Map of all GraphQL operations in the project.
 *
 * This map has several performance disadvantages:
 * 1. It is not tree-shakeable, so it will include all operations in the project.
 * 2. It is not minifiable, so the string of a GraphQL query will be multiple times inside the bundle.
 * 3. It does not support dead code elimination, so it will add unused operations.
 *
 * Therefore it is highly recommended to use the babel or swc plugin for production.
 * Learn more about it here: https://the-guild.dev/graphql/codegen/plugins/presets/preset-client#reducing-bundle-size
 */
type Documents = {
    "\n  mutation StartSession($input: StartSessionInput!) {\n    startSession(input: $input) {\n      sessionId\n      token\n      tunnelPath\n      expiresAt\n    }\n  }\n": typeof types.StartSessionDocument,
    "\n  mutation EndSession($id: ID!) {\n    endSession(id: $id) {\n      id\n      deviceId\n      deviceName\n      protocol\n      startedAt\n      endedAt\n      endReason\n    }\n  }\n": typeof types.EndSessionDocument,
    "\n  mutation TestConnection($deviceId: ID!) {\n    testConnection(input: { deviceId: $deviceId }) {\n      reachable\n      authenticated\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n      hostKeyChanged\n      message\n    }\n  }\n": typeof types.TestConnectionDocument,
    "\n  mutation AcceptHostKey($deviceId: ID!, $fingerprintSha256: String!) {\n    acceptHostKey(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {\n      algorithm\n      fingerprintSha256\n      firstSeenAt\n    }\n  }\n": typeof types.AcceptHostKeyDocument,
};
const documents: Documents = {
    "\n  mutation StartSession($input: StartSessionInput!) {\n    startSession(input: $input) {\n      sessionId\n      token\n      tunnelPath\n      expiresAt\n    }\n  }\n": types.StartSessionDocument,
    "\n  mutation EndSession($id: ID!) {\n    endSession(id: $id) {\n      id\n      deviceId\n      deviceName\n      protocol\n      startedAt\n      endedAt\n      endReason\n    }\n  }\n": types.EndSessionDocument,
    "\n  mutation TestConnection($deviceId: ID!) {\n    testConnection(input: { deviceId: $deviceId }) {\n      reachable\n      authenticated\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n      hostKeyChanged\n      message\n    }\n  }\n": types.TestConnectionDocument,
    "\n  mutation AcceptHostKey($deviceId: ID!, $fingerprintSha256: String!) {\n    acceptHostKey(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {\n      algorithm\n      fingerprintSha256\n      firstSeenAt\n    }\n  }\n": types.AcceptHostKeyDocument,
};

/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 *
 *
 * @example
 * ```ts
 * const query = graphql(`query GetUser($id: ID!) { user(id: $id) { name } }`);
 * ```
 *
 * The query argument is unknown!
 * Please regenerate the types.
 */
export function graphql(source: string): unknown;

/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation StartSession($input: StartSessionInput!) {\n    startSession(input: $input) {\n      sessionId\n      token\n      tunnelPath\n      expiresAt\n    }\n  }\n"): (typeof documents)["\n  mutation StartSession($input: StartSessionInput!) {\n    startSession(input: $input) {\n      sessionId\n      token\n      tunnelPath\n      expiresAt\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation EndSession($id: ID!) {\n    endSession(id: $id) {\n      id\n      deviceId\n      deviceName\n      protocol\n      startedAt\n      endedAt\n      endReason\n    }\n  }\n"): (typeof documents)["\n  mutation EndSession($id: ID!) {\n    endSession(id: $id) {\n      id\n      deviceId\n      deviceName\n      protocol\n      startedAt\n      endedAt\n      endReason\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation TestConnection($deviceId: ID!) {\n    testConnection(input: { deviceId: $deviceId }) {\n      reachable\n      authenticated\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n      hostKeyChanged\n      message\n    }\n  }\n"): (typeof documents)["\n  mutation TestConnection($deviceId: ID!) {\n    testConnection(input: { deviceId: $deviceId }) {\n      reachable\n      authenticated\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n      hostKeyChanged\n      message\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation AcceptHostKey($deviceId: ID!, $fingerprintSha256: String!) {\n    acceptHostKey(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {\n      algorithm\n      fingerprintSha256\n      firstSeenAt\n    }\n  }\n"): (typeof documents)["\n  mutation AcceptHostKey($deviceId: ID!, $fingerprintSha256: String!) {\n    acceptHostKey(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {\n      algorithm\n      fingerprintSha256\n      firstSeenAt\n    }\n  }\n"];

export function graphql(source: string) {
  return (documents as any)[source] ?? {};
}

export type DocumentType<TDocumentNode extends DocumentNode<any, any>> = TDocumentNode extends DocumentNode<  infer TType,  any>  ? TType  : never;