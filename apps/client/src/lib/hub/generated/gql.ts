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
    "\n  mutation Login($input: LoginInput!) {\n    login(input: $input) {\n      token\n    }\n  }\n": typeof types.LoginDocument,
    "\n  mutation Logout {\n    logout\n  }\n": typeof types.LogoutDocument,
    "\n  query Me {\n    me {\n      username\n    }\n  }\n": typeof types.MeDocument,
    "\n  subscription HeartbeatAdded {\n    heartbeatAdded(deviceId: null) {\n      checkId\n      deviceId\n      heartbeat {\n        at\n        status\n        responseTimeMs\n      }\n    }\n  }\n": typeof types.HeartbeatAddedDocument,
    "\n  fragment CheckFields on Check {\n    id\n    parent {\n      id\n    }\n    type\n    name\n    intervalSeconds\n    retries\n    retryIntervalSeconds\n    degradedThresholdMs\n    invert\n    paused\n    status\n    certificateDaysLeft\n    lastHeartbeat {\n      at\n      status\n      responseTimeMs\n      message\n    }\n    config {\n      __typename\n      ... on IcmpConfig {\n        timeoutMs\n        ipVersion\n      }\n      ... on TcpConfig {\n        port\n        timeoutMs\n      }\n      ... on TlsConfig {\n        port\n        serverName\n        timeoutMs\n      }\n      ... on HttpConfig {\n        url\n        method\n        headers {\n          name\n          value\n        }\n        body\n        maxRedirects\n        acceptedStatus {\n          from\n          to\n        }\n        keyword\n        jsonQuery\n        jsonExpected\n        authKind\n        verifyTls\n        timeoutMs\n      }\n      ... on DnsConfig {\n        name\n        recordType\n        resolver\n        expected\n      }\n      ... on PushConfig {\n        pushPath\n      }\n      ... on GroupConfig {\n        children {\n          id\n          name\n        }\n      }\n    }\n  }\n": typeof types.CheckFieldsFragmentDoc,
    "\n  query DeviceChecks($deviceId: ID!) {\n    device(id: $deviceId) {\n      id\n      checks {\n        ...CheckFields\n      }\n    }\n  }\n": typeof types.DeviceChecksDocument,
    "\n  query CheckHeartbeats($id: ID!, $first: Int, $after: String) {\n    check(id: $id) {\n      id\n      heartbeats(first: $first, after: $after) {\n        pageInfo {\n          hasNextPage\n          endCursor\n        }\n        edges {\n          node {\n            at\n            status\n            responseTimeMs\n            message\n          }\n        }\n      }\n    }\n  }\n": typeof types.CheckHeartbeatsDocument,
    "\n  mutation CreateCheck($input: CreateCheckInput!) {\n    createCheck(input: $input) {\n      ...CheckFields\n    }\n  }\n": typeof types.CreateCheckDocument,
    "\n  mutation UpdateCheck($id: ID!, $input: UpdateCheckInput!) {\n    updateCheck(id: $id, input: $input) {\n      ...CheckFields\n    }\n  }\n": typeof types.UpdateCheckDocument,
    "\n  mutation DeleteCheck($id: ID!) {\n    deleteCheck(id: $id)\n  }\n": typeof types.DeleteCheckDocument,
    "\n  mutation SetCheckPaused($id: ID!, $paused: Boolean!) {\n    setCheckPaused(id: $id, paused: $paused) {\n      ...CheckFields\n    }\n  }\n": typeof types.SetCheckPausedDocument,
    "\n  query Devices($filter: DeviceFilter, $sort: DeviceSort, $first: Int, $after: String) {\n    devices(filter: $filter, sort: $sort, first: $first, after: $after) {\n      totalCount\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          name\n          host\n          ip\n          tags\n          maintenance\n          status\n          uptime(window: DAY)\n          ssh {\n            enabled\n            hostKey {\n              fingerprintSha256\n            }\n          }\n          checks {\n            id\n            heartbeats(first: 50) {\n              edges {\n                node {\n                  at\n                  status\n                  responseTimeMs\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  }\n": typeof types.DevicesDocument,
    "\n  query Tags {\n    tags\n  }\n": typeof types.TagsDocument,
    "\n  subscription DeviceStatusChanged {\n    deviceStatusChanged(deviceId: null) {\n      deviceId\n      status\n      at\n    }\n  }\n": typeof types.DeviceStatusChangedDocument,
    "\n  fragment DeviceDetailFields on Device {\n    id\n    name\n    host\n    ip\n    kind\n    os\n    tags\n    maintenance\n    status\n    site {\n      id\n      name\n    }\n    ssh {\n      enabled\n      port\n      username\n      auth\n      credentialStored\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n    }\n    day: uptime(window: DAY)\n    week: uptime(window: WEEK)\n    month: uptime(window: MONTH)\n  }\n": typeof types.DeviceDetailFieldsFragmentDoc,
    "\n  query Device($id: ID!) {\n    device(id: $id) {\n      ...DeviceDetailFields\n    }\n  }\n": typeof types.DeviceDocument,
    "\n  query Sites {\n    sites {\n      id\n      name\n    }\n  }\n": typeof types.SitesDocument,
    "\n  mutation CreateDevice($input: CreateDeviceInput!) {\n    createDevice(input: $input) {\n      ...DeviceDetailFields\n    }\n  }\n": typeof types.CreateDeviceDocument,
    "\n  mutation UpdateDevice($id: ID!, $input: UpdateDeviceInput!) {\n    updateDevice(id: $id, input: $input) {\n      ...DeviceDetailFields\n    }\n  }\n": typeof types.UpdateDeviceDocument,
    "\n  mutation DeleteDevice($id: ID!) {\n    deleteDevice(id: $id)\n  }\n": typeof types.DeleteDeviceDocument,
    "\n  mutation SetSshCredential($deviceId: ID!, $credential: SshCredentialInput!) {\n    setSshCredential(deviceId: $deviceId, credential: $credential) {\n      credentialStored\n    }\n  }\n": typeof types.SetSshCredentialDocument,
    "\n  mutation ClearSshCredential($deviceId: ID!) {\n    clearSshCredential(deviceId: $deviceId) {\n      credentialStored\n    }\n  }\n": typeof types.ClearSshCredentialDocument,
    "\n  query Events($filter: EventFilter, $first: Int, $after: String) {\n    events(filter: $filter, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          at\n          severity\n          source\n          deviceId\n          deviceName\n          message\n        }\n      }\n    }\n  }\n": typeof types.EventsDocument,
    "\n  subscription EventAdded($minSeverity: Severity) {\n    eventAdded(minSeverity: $minSeverity) {\n      id\n      at\n      severity\n      source\n      deviceId\n      deviceName\n      message\n    }\n  }\n": typeof types.EventAddedDocument,
    "\n  mutation StartSession($input: StartSessionInput!) {\n    startSession(input: $input) {\n      sessionId\n      token\n      tunnelPath\n      expiresAt\n    }\n  }\n": typeof types.StartSessionDocument,
    "\n  mutation EndSession($id: ID!) {\n    endSession(id: $id) {\n      id\n      deviceId\n      deviceName\n      protocol\n      startedAt\n      endedAt\n      endReason\n    }\n  }\n": typeof types.EndSessionDocument,
    "\n  mutation TestConnection($input: ConnectionTestInput!) {\n    testConnection(input: $input) {\n      reachable\n      authenticated\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n      hostKeyChanged\n      message\n    }\n  }\n": typeof types.TestConnectionDocument,
    "\n  mutation AcceptHostKey($deviceId: ID!, $fingerprintSha256: String!) {\n    acceptHostKey(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {\n      algorithm\n      fingerprintSha256\n      firstSeenAt\n    }\n  }\n": typeof types.AcceptHostKeyDocument,
    "\n  query DeviceSessions($deviceId: ID!, $first: Int, $after: String) {\n    sessions(deviceId: $deviceId, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          protocol\n          operatorName\n          startedAt\n          endedAt\n          endReason\n        }\n      }\n    }\n  }\n": typeof types.DeviceSessionsDocument,
};
const documents: Documents = {
    "\n  mutation Login($input: LoginInput!) {\n    login(input: $input) {\n      token\n    }\n  }\n": types.LoginDocument,
    "\n  mutation Logout {\n    logout\n  }\n": types.LogoutDocument,
    "\n  query Me {\n    me {\n      username\n    }\n  }\n": types.MeDocument,
    "\n  subscription HeartbeatAdded {\n    heartbeatAdded(deviceId: null) {\n      checkId\n      deviceId\n      heartbeat {\n        at\n        status\n        responseTimeMs\n      }\n    }\n  }\n": types.HeartbeatAddedDocument,
    "\n  fragment CheckFields on Check {\n    id\n    parent {\n      id\n    }\n    type\n    name\n    intervalSeconds\n    retries\n    retryIntervalSeconds\n    degradedThresholdMs\n    invert\n    paused\n    status\n    certificateDaysLeft\n    lastHeartbeat {\n      at\n      status\n      responseTimeMs\n      message\n    }\n    config {\n      __typename\n      ... on IcmpConfig {\n        timeoutMs\n        ipVersion\n      }\n      ... on TcpConfig {\n        port\n        timeoutMs\n      }\n      ... on TlsConfig {\n        port\n        serverName\n        timeoutMs\n      }\n      ... on HttpConfig {\n        url\n        method\n        headers {\n          name\n          value\n        }\n        body\n        maxRedirects\n        acceptedStatus {\n          from\n          to\n        }\n        keyword\n        jsonQuery\n        jsonExpected\n        authKind\n        verifyTls\n        timeoutMs\n      }\n      ... on DnsConfig {\n        name\n        recordType\n        resolver\n        expected\n      }\n      ... on PushConfig {\n        pushPath\n      }\n      ... on GroupConfig {\n        children {\n          id\n          name\n        }\n      }\n    }\n  }\n": types.CheckFieldsFragmentDoc,
    "\n  query DeviceChecks($deviceId: ID!) {\n    device(id: $deviceId) {\n      id\n      checks {\n        ...CheckFields\n      }\n    }\n  }\n": types.DeviceChecksDocument,
    "\n  query CheckHeartbeats($id: ID!, $first: Int, $after: String) {\n    check(id: $id) {\n      id\n      heartbeats(first: $first, after: $after) {\n        pageInfo {\n          hasNextPage\n          endCursor\n        }\n        edges {\n          node {\n            at\n            status\n            responseTimeMs\n            message\n          }\n        }\n      }\n    }\n  }\n": types.CheckHeartbeatsDocument,
    "\n  mutation CreateCheck($input: CreateCheckInput!) {\n    createCheck(input: $input) {\n      ...CheckFields\n    }\n  }\n": types.CreateCheckDocument,
    "\n  mutation UpdateCheck($id: ID!, $input: UpdateCheckInput!) {\n    updateCheck(id: $id, input: $input) {\n      ...CheckFields\n    }\n  }\n": types.UpdateCheckDocument,
    "\n  mutation DeleteCheck($id: ID!) {\n    deleteCheck(id: $id)\n  }\n": types.DeleteCheckDocument,
    "\n  mutation SetCheckPaused($id: ID!, $paused: Boolean!) {\n    setCheckPaused(id: $id, paused: $paused) {\n      ...CheckFields\n    }\n  }\n": types.SetCheckPausedDocument,
    "\n  query Devices($filter: DeviceFilter, $sort: DeviceSort, $first: Int, $after: String) {\n    devices(filter: $filter, sort: $sort, first: $first, after: $after) {\n      totalCount\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          name\n          host\n          ip\n          tags\n          maintenance\n          status\n          uptime(window: DAY)\n          ssh {\n            enabled\n            hostKey {\n              fingerprintSha256\n            }\n          }\n          checks {\n            id\n            heartbeats(first: 50) {\n              edges {\n                node {\n                  at\n                  status\n                  responseTimeMs\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  }\n": types.DevicesDocument,
    "\n  query Tags {\n    tags\n  }\n": types.TagsDocument,
    "\n  subscription DeviceStatusChanged {\n    deviceStatusChanged(deviceId: null) {\n      deviceId\n      status\n      at\n    }\n  }\n": types.DeviceStatusChangedDocument,
    "\n  fragment DeviceDetailFields on Device {\n    id\n    name\n    host\n    ip\n    kind\n    os\n    tags\n    maintenance\n    status\n    site {\n      id\n      name\n    }\n    ssh {\n      enabled\n      port\n      username\n      auth\n      credentialStored\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n    }\n    day: uptime(window: DAY)\n    week: uptime(window: WEEK)\n    month: uptime(window: MONTH)\n  }\n": types.DeviceDetailFieldsFragmentDoc,
    "\n  query Device($id: ID!) {\n    device(id: $id) {\n      ...DeviceDetailFields\n    }\n  }\n": types.DeviceDocument,
    "\n  query Sites {\n    sites {\n      id\n      name\n    }\n  }\n": types.SitesDocument,
    "\n  mutation CreateDevice($input: CreateDeviceInput!) {\n    createDevice(input: $input) {\n      ...DeviceDetailFields\n    }\n  }\n": types.CreateDeviceDocument,
    "\n  mutation UpdateDevice($id: ID!, $input: UpdateDeviceInput!) {\n    updateDevice(id: $id, input: $input) {\n      ...DeviceDetailFields\n    }\n  }\n": types.UpdateDeviceDocument,
    "\n  mutation DeleteDevice($id: ID!) {\n    deleteDevice(id: $id)\n  }\n": types.DeleteDeviceDocument,
    "\n  mutation SetSshCredential($deviceId: ID!, $credential: SshCredentialInput!) {\n    setSshCredential(deviceId: $deviceId, credential: $credential) {\n      credentialStored\n    }\n  }\n": types.SetSshCredentialDocument,
    "\n  mutation ClearSshCredential($deviceId: ID!) {\n    clearSshCredential(deviceId: $deviceId) {\n      credentialStored\n    }\n  }\n": types.ClearSshCredentialDocument,
    "\n  query Events($filter: EventFilter, $first: Int, $after: String) {\n    events(filter: $filter, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          at\n          severity\n          source\n          deviceId\n          deviceName\n          message\n        }\n      }\n    }\n  }\n": types.EventsDocument,
    "\n  subscription EventAdded($minSeverity: Severity) {\n    eventAdded(minSeverity: $minSeverity) {\n      id\n      at\n      severity\n      source\n      deviceId\n      deviceName\n      message\n    }\n  }\n": types.EventAddedDocument,
    "\n  mutation StartSession($input: StartSessionInput!) {\n    startSession(input: $input) {\n      sessionId\n      token\n      tunnelPath\n      expiresAt\n    }\n  }\n": types.StartSessionDocument,
    "\n  mutation EndSession($id: ID!) {\n    endSession(id: $id) {\n      id\n      deviceId\n      deviceName\n      protocol\n      startedAt\n      endedAt\n      endReason\n    }\n  }\n": types.EndSessionDocument,
    "\n  mutation TestConnection($input: ConnectionTestInput!) {\n    testConnection(input: $input) {\n      reachable\n      authenticated\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n      hostKeyChanged\n      message\n    }\n  }\n": types.TestConnectionDocument,
    "\n  mutation AcceptHostKey($deviceId: ID!, $fingerprintSha256: String!) {\n    acceptHostKey(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {\n      algorithm\n      fingerprintSha256\n      firstSeenAt\n    }\n  }\n": types.AcceptHostKeyDocument,
    "\n  query DeviceSessions($deviceId: ID!, $first: Int, $after: String) {\n    sessions(deviceId: $deviceId, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          protocol\n          operatorName\n          startedAt\n          endedAt\n          endReason\n        }\n      }\n    }\n  }\n": types.DeviceSessionsDocument,
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
export function graphql(source: "\n  mutation Login($input: LoginInput!) {\n    login(input: $input) {\n      token\n    }\n  }\n"): (typeof documents)["\n  mutation Login($input: LoginInput!) {\n    login(input: $input) {\n      token\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation Logout {\n    logout\n  }\n"): (typeof documents)["\n  mutation Logout {\n    logout\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Me {\n    me {\n      username\n    }\n  }\n"): (typeof documents)["\n  query Me {\n    me {\n      username\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  subscription HeartbeatAdded {\n    heartbeatAdded(deviceId: null) {\n      checkId\n      deviceId\n      heartbeat {\n        at\n        status\n        responseTimeMs\n      }\n    }\n  }\n"): (typeof documents)["\n  subscription HeartbeatAdded {\n    heartbeatAdded(deviceId: null) {\n      checkId\n      deviceId\n      heartbeat {\n        at\n        status\n        responseTimeMs\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  fragment CheckFields on Check {\n    id\n    parent {\n      id\n    }\n    type\n    name\n    intervalSeconds\n    retries\n    retryIntervalSeconds\n    degradedThresholdMs\n    invert\n    paused\n    status\n    certificateDaysLeft\n    lastHeartbeat {\n      at\n      status\n      responseTimeMs\n      message\n    }\n    config {\n      __typename\n      ... on IcmpConfig {\n        timeoutMs\n        ipVersion\n      }\n      ... on TcpConfig {\n        port\n        timeoutMs\n      }\n      ... on TlsConfig {\n        port\n        serverName\n        timeoutMs\n      }\n      ... on HttpConfig {\n        url\n        method\n        headers {\n          name\n          value\n        }\n        body\n        maxRedirects\n        acceptedStatus {\n          from\n          to\n        }\n        keyword\n        jsonQuery\n        jsonExpected\n        authKind\n        verifyTls\n        timeoutMs\n      }\n      ... on DnsConfig {\n        name\n        recordType\n        resolver\n        expected\n      }\n      ... on PushConfig {\n        pushPath\n      }\n      ... on GroupConfig {\n        children {\n          id\n          name\n        }\n      }\n    }\n  }\n"): (typeof documents)["\n  fragment CheckFields on Check {\n    id\n    parent {\n      id\n    }\n    type\n    name\n    intervalSeconds\n    retries\n    retryIntervalSeconds\n    degradedThresholdMs\n    invert\n    paused\n    status\n    certificateDaysLeft\n    lastHeartbeat {\n      at\n      status\n      responseTimeMs\n      message\n    }\n    config {\n      __typename\n      ... on IcmpConfig {\n        timeoutMs\n        ipVersion\n      }\n      ... on TcpConfig {\n        port\n        timeoutMs\n      }\n      ... on TlsConfig {\n        port\n        serverName\n        timeoutMs\n      }\n      ... on HttpConfig {\n        url\n        method\n        headers {\n          name\n          value\n        }\n        body\n        maxRedirects\n        acceptedStatus {\n          from\n          to\n        }\n        keyword\n        jsonQuery\n        jsonExpected\n        authKind\n        verifyTls\n        timeoutMs\n      }\n      ... on DnsConfig {\n        name\n        recordType\n        resolver\n        expected\n      }\n      ... on PushConfig {\n        pushPath\n      }\n      ... on GroupConfig {\n        children {\n          id\n          name\n        }\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query DeviceChecks($deviceId: ID!) {\n    device(id: $deviceId) {\n      id\n      checks {\n        ...CheckFields\n      }\n    }\n  }\n"): (typeof documents)["\n  query DeviceChecks($deviceId: ID!) {\n    device(id: $deviceId) {\n      id\n      checks {\n        ...CheckFields\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query CheckHeartbeats($id: ID!, $first: Int, $after: String) {\n    check(id: $id) {\n      id\n      heartbeats(first: $first, after: $after) {\n        pageInfo {\n          hasNextPage\n          endCursor\n        }\n        edges {\n          node {\n            at\n            status\n            responseTimeMs\n            message\n          }\n        }\n      }\n    }\n  }\n"): (typeof documents)["\n  query CheckHeartbeats($id: ID!, $first: Int, $after: String) {\n    check(id: $id) {\n      id\n      heartbeats(first: $first, after: $after) {\n        pageInfo {\n          hasNextPage\n          endCursor\n        }\n        edges {\n          node {\n            at\n            status\n            responseTimeMs\n            message\n          }\n        }\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CreateCheck($input: CreateCheckInput!) {\n    createCheck(input: $input) {\n      ...CheckFields\n    }\n  }\n"): (typeof documents)["\n  mutation CreateCheck($input: CreateCheckInput!) {\n    createCheck(input: $input) {\n      ...CheckFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UpdateCheck($id: ID!, $input: UpdateCheckInput!) {\n    updateCheck(id: $id, input: $input) {\n      ...CheckFields\n    }\n  }\n"): (typeof documents)["\n  mutation UpdateCheck($id: ID!, $input: UpdateCheckInput!) {\n    updateCheck(id: $id, input: $input) {\n      ...CheckFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DeleteCheck($id: ID!) {\n    deleteCheck(id: $id)\n  }\n"): (typeof documents)["\n  mutation DeleteCheck($id: ID!) {\n    deleteCheck(id: $id)\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SetCheckPaused($id: ID!, $paused: Boolean!) {\n    setCheckPaused(id: $id, paused: $paused) {\n      ...CheckFields\n    }\n  }\n"): (typeof documents)["\n  mutation SetCheckPaused($id: ID!, $paused: Boolean!) {\n    setCheckPaused(id: $id, paused: $paused) {\n      ...CheckFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Devices($filter: DeviceFilter, $sort: DeviceSort, $first: Int, $after: String) {\n    devices(filter: $filter, sort: $sort, first: $first, after: $after) {\n      totalCount\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          name\n          host\n          ip\n          tags\n          maintenance\n          status\n          uptime(window: DAY)\n          ssh {\n            enabled\n            hostKey {\n              fingerprintSha256\n            }\n          }\n          checks {\n            id\n            heartbeats(first: 50) {\n              edges {\n                node {\n                  at\n                  status\n                  responseTimeMs\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  }\n"): (typeof documents)["\n  query Devices($filter: DeviceFilter, $sort: DeviceSort, $first: Int, $after: String) {\n    devices(filter: $filter, sort: $sort, first: $first, after: $after) {\n      totalCount\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          name\n          host\n          ip\n          tags\n          maintenance\n          status\n          uptime(window: DAY)\n          ssh {\n            enabled\n            hostKey {\n              fingerprintSha256\n            }\n          }\n          checks {\n            id\n            heartbeats(first: 50) {\n              edges {\n                node {\n                  at\n                  status\n                  responseTimeMs\n                }\n              }\n            }\n          }\n        }\n      }\n    }\n  }\n"];
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
export function graphql(source: "\n  fragment DeviceDetailFields on Device {\n    id\n    name\n    host\n    ip\n    kind\n    os\n    tags\n    maintenance\n    status\n    site {\n      id\n      name\n    }\n    ssh {\n      enabled\n      port\n      username\n      auth\n      credentialStored\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n    }\n    day: uptime(window: DAY)\n    week: uptime(window: WEEK)\n    month: uptime(window: MONTH)\n  }\n"): (typeof documents)["\n  fragment DeviceDetailFields on Device {\n    id\n    name\n    host\n    ip\n    kind\n    os\n    tags\n    maintenance\n    status\n    site {\n      id\n      name\n    }\n    ssh {\n      enabled\n      port\n      username\n      auth\n      credentialStored\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n    }\n    day: uptime(window: DAY)\n    week: uptime(window: WEEK)\n    month: uptime(window: MONTH)\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Device($id: ID!) {\n    device(id: $id) {\n      ...DeviceDetailFields\n    }\n  }\n"): (typeof documents)["\n  query Device($id: ID!) {\n    device(id: $id) {\n      ...DeviceDetailFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Sites {\n    sites {\n      id\n      name\n    }\n  }\n"): (typeof documents)["\n  query Sites {\n    sites {\n      id\n      name\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation CreateDevice($input: CreateDeviceInput!) {\n    createDevice(input: $input) {\n      ...DeviceDetailFields\n    }\n  }\n"): (typeof documents)["\n  mutation CreateDevice($input: CreateDeviceInput!) {\n    createDevice(input: $input) {\n      ...DeviceDetailFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation UpdateDevice($id: ID!, $input: UpdateDeviceInput!) {\n    updateDevice(id: $id, input: $input) {\n      ...DeviceDetailFields\n    }\n  }\n"): (typeof documents)["\n  mutation UpdateDevice($id: ID!, $input: UpdateDeviceInput!) {\n    updateDevice(id: $id, input: $input) {\n      ...DeviceDetailFields\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation DeleteDevice($id: ID!) {\n    deleteDevice(id: $id)\n  }\n"): (typeof documents)["\n  mutation DeleteDevice($id: ID!) {\n    deleteDevice(id: $id)\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation SetSshCredential($deviceId: ID!, $credential: SshCredentialInput!) {\n    setSshCredential(deviceId: $deviceId, credential: $credential) {\n      credentialStored\n    }\n  }\n"): (typeof documents)["\n  mutation SetSshCredential($deviceId: ID!, $credential: SshCredentialInput!) {\n    setSshCredential(deviceId: $deviceId, credential: $credential) {\n      credentialStored\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation ClearSshCredential($deviceId: ID!) {\n    clearSshCredential(deviceId: $deviceId) {\n      credentialStored\n    }\n  }\n"): (typeof documents)["\n  mutation ClearSshCredential($deviceId: ID!) {\n    clearSshCredential(deviceId: $deviceId) {\n      credentialStored\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query Events($filter: EventFilter, $first: Int, $after: String) {\n    events(filter: $filter, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          at\n          severity\n          source\n          deviceId\n          deviceName\n          message\n        }\n      }\n    }\n  }\n"): (typeof documents)["\n  query Events($filter: EventFilter, $first: Int, $after: String) {\n    events(filter: $filter, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          at\n          severity\n          source\n          deviceId\n          deviceName\n          message\n        }\n      }\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  subscription EventAdded($minSeverity: Severity) {\n    eventAdded(minSeverity: $minSeverity) {\n      id\n      at\n      severity\n      source\n      deviceId\n      deviceName\n      message\n    }\n  }\n"): (typeof documents)["\n  subscription EventAdded($minSeverity: Severity) {\n    eventAdded(minSeverity: $minSeverity) {\n      id\n      at\n      severity\n      source\n      deviceId\n      deviceName\n      message\n    }\n  }\n"];
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
export function graphql(source: "\n  mutation TestConnection($input: ConnectionTestInput!) {\n    testConnection(input: $input) {\n      reachable\n      authenticated\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n      hostKeyChanged\n      message\n    }\n  }\n"): (typeof documents)["\n  mutation TestConnection($input: ConnectionTestInput!) {\n    testConnection(input: $input) {\n      reachable\n      authenticated\n      hostKey {\n        algorithm\n        fingerprintSha256\n        firstSeenAt\n      }\n      hostKeyChanged\n      message\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  mutation AcceptHostKey($deviceId: ID!, $fingerprintSha256: String!) {\n    acceptHostKey(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {\n      algorithm\n      fingerprintSha256\n      firstSeenAt\n    }\n  }\n"): (typeof documents)["\n  mutation AcceptHostKey($deviceId: ID!, $fingerprintSha256: String!) {\n    acceptHostKey(deviceId: $deviceId, fingerprintSha256: $fingerprintSha256) {\n      algorithm\n      fingerprintSha256\n      firstSeenAt\n    }\n  }\n"];
/**
 * The graphql function is used to parse GraphQL queries into a document that can be used by GraphQL clients.
 */
export function graphql(source: "\n  query DeviceSessions($deviceId: ID!, $first: Int, $after: String) {\n    sessions(deviceId: $deviceId, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          protocol\n          operatorName\n          startedAt\n          endedAt\n          endReason\n        }\n      }\n    }\n  }\n"): (typeof documents)["\n  query DeviceSessions($deviceId: ID!, $first: Int, $after: String) {\n    sessions(deviceId: $deviceId, first: $first, after: $after) {\n      pageInfo {\n        hasNextPage\n        endCursor\n      }\n      edges {\n        node {\n          id\n          protocol\n          operatorName\n          startedAt\n          endedAt\n          endReason\n        }\n      }\n    }\n  }\n"];

export function graphql(source: string) {
  return (documents as any)[source] ?? {};
}

export type DocumentType<TDocumentNode extends DocumentNode<any, any>> = TDocumentNode extends DocumentNode<  infer TType,  any>  ? TType  : never;