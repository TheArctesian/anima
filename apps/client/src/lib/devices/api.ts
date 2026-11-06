import { graphql } from '$lib/hub/generated';
import type {
  CreateDeviceInput,
  DeviceHostsQuery,
  DeviceQuery,
  DevicesQuery,
  DevicesQueryVariables,
  SetSshCredentialMutation,
  SitesQuery,
  SshCredentialInput,
  UpdateDeviceInput,
} from '$lib/hub/generated/graphql';
import type { Hub, HubResult, Sink } from '$lib/hub/hub';

const Devices = graphql(`
  query Devices($filter: DeviceFilter, $sort: DeviceSort, $first: Int, $after: String) {
    devices(filter: $filter, sort: $sort, first: $first, after: $after) {
      totalCount
      pageInfo {
        hasNextPage
        endCursor
      }
      edges {
        node {
          id
          name
          host
          ip
          tags
          maintenance
          status
          uptime(window: DAY)
          ssh {
            enabled
            hostKey {
              fingerprintSha256
            }
          }
          checks {
            id
            heartbeats(first: 50) {
              edges {
                node {
                  at
                  status
                  responseTimeMs
                }
              }
            }
          }
        }
      }
    }
  }
`);

const DeviceHosts = graphql(`
  query DeviceHosts($after: String) {
    devices(first: 200, after: $after) {
      pageInfo {
        hasNextPage
        endCursor
      }
      edges {
        node {
          host
        }
      }
    }
  }
`);

/** At most this many pages of 200 hosts are read: the hub's page limit times a bound no home lab reaches. */
const MAX_HOST_PAGES = 50;

const Tags = graphql(`
  query Tags {
    tags
  }
`);

const DeviceStatusChanged = graphql(`
  subscription DeviceStatusChanged {
    deviceStatusChanged(deviceId: null) {
      deviceId
      status
      at
    }
  }
`);

// The fragment is only read by codegen, which inlines it into the documents that spread it.
void graphql(`
  fragment DeviceDetailFields on Device {
    id
    name
    host
    ip
    kind
    os
    tags
    maintenance
    status
    site {
      id
      name
    }
    ssh {
      enabled
      port
      username
      auth
      credentialStored
      hostKey {
        algorithm
        fingerprintSha256
        firstSeenAt
      }
    }
    day: uptime(window: DAY)
    week: uptime(window: WEEK)
    month: uptime(window: MONTH)
  }
`);

const Device = graphql(`
  query Device($id: ID!) {
    device(id: $id) {
      ...DeviceDetailFields
    }
  }
`);

const Sites = graphql(`
  query Sites {
    sites {
      id
      name
    }
  }
`);

const CreateDevice = graphql(`
  mutation CreateDevice($input: CreateDeviceInput!) {
    createDevice(input: $input) {
      ...DeviceDetailFields
    }
  }
`);

const UpdateDevice = graphql(`
  mutation UpdateDevice($id: ID!, $input: UpdateDeviceInput!) {
    updateDevice(id: $id, input: $input) {
      ...DeviceDetailFields
    }
  }
`);

const DeleteDevice = graphql(`
  mutation DeleteDevice($id: ID!) {
    deleteDevice(id: $id)
  }
`);

const SetSshCredential = graphql(`
  mutation SetSshCredential($deviceId: ID!, $credential: SshCredentialInput!) {
    setSshCredential(deviceId: $deviceId, credential: $credential) {
      credentialStored
    }
  }
`);

const ClearSshCredential = graphql(`
  mutation ClearSshCredential($deviceId: ID!) {
    clearSshCredential(deviceId: $deviceId) {
      credentialStored
    }
  }
`);

/** One page of the device list, as the hub answers it. */
export type DevicePage = DevicesQuery['devices'];

/** A device with every field a card or table row shows. */
export type CardDevice = DevicePage['edges'][number]['node'];

/** One device with everything the detail page and the device form show (DEV-020, D-10). */
export type DeviceDetail = NonNullable<DeviceQuery['device']>;

/** A site the Add device form can place a device in. */
export type Site = SitesQuery['sites'][number];

/** The SSH settings after a credential change: `credentialStored` says whether the hub holds a secret now. */
export type StoredCredential = SetSshCredentialMutation['setSshCredential'];

/** A device's new status, as the hub pushes it (DEV-003). */
export interface StatusChange {
  readonly deviceId: string;
  readonly status: CardDevice['status'];
  readonly at: string;
}

/** Every device's host, page by page. Stops at the last page, at a page without a cursor, or after `MAX_HOST_PAGES`. */
async function readHosts(hub: Hub): Promise<HubResult<readonly string[]>> {
  const hosts: string[] = [];
  let after: string | null = null;
  for (let page = 0; page < MAX_HOST_PAGES; page += 1) {
    const result: HubResult<DeviceHostsQuery> = await hub.request(DeviceHosts, { after });
    if (!result.ok) return result;
    const { edges, pageInfo }: DeviceHostsQuery['devices'] = result.value.devices;
    hosts.push(...edges.map((edge) => edge.node.host));
    after = pageInfo.endCursor ?? null;
    if (!pageInfo.hasNextPage || after === null) break;
  }
  return { ok: true, value: hosts };
}

/** The device list, tags and live status changes through a `Hub` (CQ-045). */
export function devicesApi(hub: Hub) {
  return {
    list: async (variables: DevicesQueryVariables): Promise<HubResult<DevicePage>> => {
      const result = await hub.request(Devices, variables);
      return result.ok ? { ok: true, value: result.value.devices } : result;
    },
    /** Every device's host (R-060: is this computer already a device?). */
    hosts: (): Promise<HubResult<readonly string[]>> => readHosts(hub),
    tags: async (): Promise<HubResult<readonly string[]>> => {
      const result = await hub.request(Tags, {});
      return result.ok ? { ok: true, value: result.value.tags } : result;
    },
    /** One device by ID; `null` when the hub has none (DEV-020). */
    get: async (id: string): Promise<HubResult<DeviceDetail | null>> => {
      const result = await hub.request(Device, { id });
      return result.ok ? { ok: true, value: result.value.device ?? null } : result;
    },
    sites: async (): Promise<HubResult<readonly Site[]>> => {
      const result = await hub.request(Sites, {});
      return result.ok ? { ok: true, value: result.value.sites } : result;
    },
    create: async (input: CreateDeviceInput): Promise<HubResult<DeviceDetail>> => {
      const result = await hub.request(CreateDevice, { input });
      return result.ok ? { ok: true, value: result.value.createDevice } : result;
    },
    update: async (id: string, input: UpdateDeviceInput): Promise<HubResult<DeviceDetail>> => {
      const result = await hub.request(UpdateDevice, { id, input });
      return result.ok ? { ok: true, value: result.value.updateDevice } : result;
    },
    /** Deletes the device with its credentials, checks and history (DEV-015); resolves to the deleted ID. */
    delete: async (id: string): Promise<HubResult<string>> => {
      const result = await hub.request(DeleteDevice, { id });
      return result.ok ? { ok: true, value: result.value.deleteDevice } : result;
    },
    setSshCredential: async (deviceId: string, credential: SshCredentialInput): Promise<HubResult<StoredCredential>> => {
      const result = await hub.request(SetSshCredential, { deviceId, credential });
      return result.ok ? { ok: true, value: result.value.setSshCredential } : result;
    },
    clearSshCredential: async (deviceId: string): Promise<HubResult<StoredCredential>> => {
      const result = await hub.request(ClearSshCredential, { deviceId });
      return result.ok ? { ok: true, value: result.value.clearSshCredential } : result;
    },
    /** Listen for status changes of every device; returns the function that stops listening. */
    watchStatus: (sink: Sink<StatusChange>): (() => void) =>
      hub.subscribe(
        DeviceStatusChanged,
        {},
        {
          next: (value) => {
            sink.next(value.deviceStatusChanged);
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
