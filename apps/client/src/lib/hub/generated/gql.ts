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
    "\n  query Devices($filter: DeviceFilter, $sort: DeviceSort, $first: Int, $after: String) {\n    devices(filter: $filter, sort: $sort, first: $first, after: $after) {\n      totalCount\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          name\n          host\n          ip\n          tags\n          maintenance\n          status\n          uptime(window: DAY)\n          ssh {\n            enabled\n          }\n          checks {\n            id\n            heartbeats(first: 50) {\n              edges {\n                node {\n                  at\n                  status\n                  responseTimeMs\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  }\n": typeof types.DevicesDocument,
    "\n  query Tags {\n    tags\n  }\n": typeof types.TagsDocument,
    "\n  subscription DeviceStatusChanged {\n    deviceStatusChanged(deviceId: null) {\n      deviceId\n      status\n      at\n    }\n  }\n": typeof types.DeviceStatusChangedDocument,
    "\n  query Events($filter: EventFilter, $first: Int, $after: String) {\n    events(filter: $filter, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          at\n          severity\n          source\n          deviceId\n          deviceName\n          message\n        }\n      }\n    }\n  }\n": typeof types.EventsDocument,
    "\n  subscription EventAdded($minSeverity: Severity) {\n    eventAdded(minSeverity: $minSeverity) {\n      id\n      at\n      severity\n      source\n      deviceId\n      deviceName\n      message\n    }\n  }\n": typeof types.EventAddedDocument,
};
const documents: Documents = {
    "\n  query Devices($filter: DeviceFilter, $sort: DeviceSort, $first: Int, $after: String) {\n    devices(filter: $filter, sort: $sort, first: $first, after: $after) {\n      totalCount\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          name\n          host\n          ip\n          tags\n          maintenance\n          status\n          uptime(window: DAY)\n          ssh {\n            enabled\n          }\n          checks {\n            id\n            heartbeats(first: 50) {\n              edges {\n                node {\n                  at\n                  status\n                  responseTimeMs\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  }\n": types.DevicesDocument,
    "\n  query Tags {\n    tags\n  }\n": types.TagsDocument,
    "\n  subscription DeviceStatusChanged {\n    deviceStatusChanged(deviceId: null) {\n      deviceId\n      status\n      at\n    }\n  }\n": types.DeviceStatusChangedDocument,
    "\n  query Events($filter: EventFilter, $first: Int, $after: String) {\n    events(filter: $filter, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          at\n          severity\n          source\n          deviceId\n          deviceName\n          message\n        }\n      }\n    }\n  }\n": types.EventsDocument,
    "\n  subscription EventAdded($minSeverity: Severity) {\n    eventAdded(minSeverity: $minSeverity) {\n      id\n      at\n      severity\n      source\n      deviceId\n      deviceName\n      message\n    }\n  }\n": types.EventAddedDocument,
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
export function graphql(source: "\n  query Devices($filter: DeviceFilter, $sort: DeviceSort, $first: Int, $after: String) {\n    devices(filter: $filter, sort: $sort, first: $first, after: $after) {\n      totalCount\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          name\n          host\n          ip\n          tags\n          maintenance\n          status\n          uptime(window: DAY)\n          ssh {\n            enabled\n          }\n          checks {\n            id\n            heartbeats(first: 50) {\n              edges {\n                node {\n                  at\n                  status\n                  responseTimeMs\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  }\n"): (typeof documents)["\n  query Devices($filter: DeviceFilter, $sort: DeviceSort, $first: Int, $after: String) {\n    devices(filter: $filter, sort: $sort, first: $first, after: $after) {\n      totalCount\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          name\n          host\n          ip\n          tags\n          maintenance\n          status\n          uptime(window: DAY)\n          ssh {\n            enabled\n          }\n          checks {\n            id\n            heartbeats(first: 50) {\n              edges {\n                node {\n                  at\n                  status\n                  responseTimeMs\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Tags {\n    tags\n  }\n"): (typeof documents)["\n  query Tags {\n    tags\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  subscription DeviceStatusChanged {\n    deviceStatusChanged(deviceId: null) {\n      deviceId\n      status\n      at\n    }\n  }\n"): (typeof documents)["\n  subscription DeviceStatusChanged {\n    deviceStatusChanged(deviceId: null) {\n      deviceId\n      status\n      at\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Events($filter: EventFilter, $first: Int, $after: String) {\n    events(filter: $filter, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          at\n          severity\n          source\n          deviceId\n          deviceName\n          message\n        }\n      }\n    }\n  }\n"): (typeof documents)["\n  query Events($filter: EventFilter, $first: Int, $after: String) {\n    events(filter: $filter, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          at\n          severity\n          source\n          deviceId\n          deviceName\n          message\n        }\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  subscription EventAdded($minSeverity: Severity) {\n    eventAdded(minSeverity: $minSeverity) {\n      id\n      at\n      severity\n      source\n      deviceId\n      deviceName\n      message\n    }\n  }\n"): (typeof documents)["\n  subscription EventAdded($minSeverity: Severity) {\n    eventAdded(minSeverity: $minSeverity) {\n      id\n      at\n      severity\n      source\n      deviceId\n      deviceName\n      message\n    }\n  }\n"];

export function graphql(source: string) {
  return (documents as any)[source] ?? {};
}

export type DocumentType<TDocumentNode extends DocumentNode<any, any>> = TDocumentNode extends DocumentNode<  infer TType,  any>  ? TType  : never;