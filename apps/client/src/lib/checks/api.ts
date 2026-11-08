import { graphql } from '$lib/hub/generated';
import type {
  CheckDefaultsInput,
  CheckDefaultsQuery,
  CheckHeartbeatsQuery,
  CreateCheckInput,
  DeviceChecksQuery,
  HeartbeatAddedSubscription,
  UpdateCheckInput,
} from '$lib/hub/generated/graphql';
import type { Hub, HubResult, Sink } from '$lib/hub/hub';

const HeartbeatAdded = graphql(`
  subscription HeartbeatAdded {
    heartbeatAdded(deviceId: null) {
      checkId
      deviceId
      heartbeat {
        at
        status
        responseTimeMs
      }
    }
  }
`);

// The fragment is only read by codegen, which inlines it into the documents that spread it.
void graphql(`
  fragment CheckFields on Check {
    id
    parent {
      id
    }
    type
    name
    intervalSeconds
    retries
    retryIntervalSeconds
    degradedThresholdMs
    invert
    paused
    status
    certificateDaysLeft
    lastHeartbeat {
      at
      status
      responseTimeMs
      message
    }
    config {
      __typename
      ... on IcmpConfig {
        timeoutMs
        ipVersion
      }
      ... on TcpConfig {
        port
        timeoutMs
      }
      ... on TlsConfig {
        port
        serverName
        timeoutMs
      }
      ... on HttpConfig {
        url
        method
        headers {
          name
          value
        }
        body
        maxRedirects
        acceptedStatus {
          from
          to
        }
        keyword
        jsonQuery
        jsonExpected
        authKind
        verifyTls
        timeoutMs
      }
      ... on DnsConfig {
        name
        recordType
        resolver
        expected
      }
      ... on PushConfig {
        pushPath
      }
      ... on GroupConfig {
        children {
          id
          name
        }
      }
    }
  }
`);

// The Phase 2 config shapes (CHK-009). Not spread into any document yet: the hub declares these types with
// WP-29, and a query that selected them earlier would fail validation. Codegen reads them for the form's types.
void graphql(`
  fragment DockerCheckConfig on DockerConfig {
    __typename
    container
    timeoutMs
  }
`);

void graphql(`
  fragment SystemdCheckConfig on SystemdConfig {
    __typename
    unit
    timeoutMs
  }
`);

void graphql(`
  fragment SnmpCheckConfig on SnmpConfig {
    __typename
    port
    version
    oid
    expected
    username
    timeoutMs
  }
`);

void graphql(`
  fragment TailscalePingCheckConfig on TailscalePingConfig {
    __typename
    target
    timeoutMs
  }
`);

// Own documents (WP-50 F-3): the K1 default fields are @pending, so a hub without them fails these two documents only.
const CheckDefaultsDocument = graphql(`
  query CheckDefaults {
    checkDefaults {
      intervalSeconds
      retries
      retryIntervalSeconds
      degradedThresholdMs
    }
  }
`);

const SetCheckDefaultsDocument = graphql(`
  mutation SetCheckDefaults($input: CheckDefaultsInput!) {
    setCheckDefaults(input: $input) {
      intervalSeconds
      retries
      retryIntervalSeconds
      degradedThresholdMs
    }
  }
`);

const DeviceChecks = graphql(`
  query DeviceChecks($deviceId: ID!) {
    device(id: $deviceId) {
      id
      checks {
        ...CheckFields
      }
    }
  }
`);

const CheckHeartbeats = graphql(`
  query CheckHeartbeats($id: ID!, $first: Int, $after: String) {
    check(id: $id) {
      id
      heartbeats(first: $first, after: $after) {
        pageInfo {
          hasNextPage
          endCursor
        }
        edges {
          node {
            at
            status
            responseTimeMs
            message
          }
        }
      }
    }
  }
`);

const CreateCheck = graphql(`
  mutation CreateCheck($input: CreateCheckInput!) {
    createCheck(input: $input) {
      ...CheckFields
    }
  }
`);

const UpdateCheck = graphql(`
  mutation UpdateCheck($id: ID!, $input: UpdateCheckInput!) {
    updateCheck(id: $id, input: $input) {
      ...CheckFields
    }
  }
`);

const DeleteCheck = graphql(`
  mutation DeleteCheck($id: ID!) {
    deleteCheck(id: $id)
  }
`);

const SetCheckPaused = graphql(`
  mutation SetCheckPaused($id: ID!, $paused: Boolean!) {
    setCheckPaused(id: $id, paused: $paused) {
      ...CheckFields
    }
  }
`);

/** A heartbeat the hub just recorded, with the check and device it belongs to. */
export type NewHeartbeat = HeartbeatAddedSubscription['heartbeatAdded'];

/** A check with its full configuration, as the check list and the check form need it (CHK-005, CHK-007, Section 6.4). */
export type DeviceCheck = NonNullable<DeviceChecksQuery['device']>['checks'][number];

/** The values a new check starts with (D-26). */
export type CheckDefaults = CheckDefaultsQuery['checkDefaults'];

/** One page of a check's heartbeats, newest first. */
export type HeartbeatPage = NonNullable<CheckHeartbeatsQuery['check']>['heartbeats'];

/** Checks and their heartbeats through a `Hub` (CQ-045). */
export function checksApi(hub: Hub) {
  return {
    /** The device's checks; `null` when the hub has no such device. */
    forDevice: async (deviceId: string): Promise<HubResult<readonly DeviceCheck[] | null>> => {
      const result = await hub.request(DeviceChecks, { deviceId });
      return result.ok ? { ok: true, value: result.value.device?.checks ?? null } : result;
    },
    /** One page of a check's heartbeats; pass the previous `endCursor` as `after` for older ones. `null` when the check is gone. */
    heartbeats: async (id: string, first: number, after: string | null): Promise<HubResult<HeartbeatPage | null>> => {
      const result = await hub.request(CheckHeartbeats, { id, first, after });
      return result.ok ? { ok: true, value: result.value.check?.heartbeats ?? null } : result;
    },
    create: async (input: CreateCheckInput): Promise<HubResult<DeviceCheck>> => {
      const result = await hub.request(CreateCheck, { input });
      return result.ok ? { ok: true, value: result.value.createCheck } : result;
    },
    update: async (id: string, input: UpdateCheckInput): Promise<HubResult<DeviceCheck>> => {
      const result = await hub.request(UpdateCheck, { id, input });
      return result.ok ? { ok: true, value: result.value.updateCheck } : result;
    },
    /** Resolves to the deleted ID; a group's children are the hub's to handle. */
    delete: async (id: string): Promise<HubResult<string>> => {
      const result = await hub.request(DeleteCheck, { id });
      return result.ok ? { ok: true, value: result.value.deleteCheck } : result;
    },
    /** Pause or resume a check (CHK-005). */
    setPaused: async (id: string, paused: boolean): Promise<HubResult<DeviceCheck>> => {
      const result = await hub.request(SetCheckPaused, { id, paused });
      return result.ok ? { ok: true, value: result.value.setCheckPaused } : result;
    },
    /** Listen for new heartbeats of every device; returns the function that stops listening. */
    watchHeartbeats: (sink: Sink<NewHeartbeat>): (() => void) =>
      hub.subscribe(
        HeartbeatAdded,
        {},
        {
          next: (value) => {
            sink.next(value.heartbeatAdded);
          },
          error: (error) => {
            sink.error(error);
          },
          complete: () => {
            sink.complete();
          },
        },
      ),
  };
}

/** The default check settings of the hub (D-26) as plain functions of a `Hub`. */
export function checkDefaultsApi(hub: Hub) {
  return {
    get: async (): Promise<HubResult<CheckDefaults>> => {
      const result = await hub.request(CheckDefaultsDocument, {});
      return result.ok ? { ok: true, value: result.value.checkDefaults } : result;
    },
    /** The defaults as the hub now holds them. */
    set: async (input: CheckDefaultsInput): Promise<HubResult<CheckDefaults>> => {
      const result = await hub.request(SetCheckDefaultsDocument, { input });
      return result.ok ? { ok: true, value: result.value.setCheckDefaults } : result;
    },
  };
}
