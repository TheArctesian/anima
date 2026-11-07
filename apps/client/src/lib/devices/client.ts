import type {
  ClearRdpCredentialMutation,
  ClearVncCredentialMutation,
  CreateDeviceInput,
  DeviceHostsQuery,
  DevicesQueryVariables,
  SshCredentialInput,
  UpdateDeviceInput,
} from '$lib/hub/generated/graphql';
import type { Hub, HubError, HubResult, Sink } from '$lib/hub/hub';
import {
  ClearRdpCredential,
  ClearSshCredential,
  ClearVncCredential,
  CardReadingsAdded,
  CreateDevice,
  CreateSite,
  DeleteDevice,
  DeviceHosts,
  Device,
  Devices,
  DeviceStatusChanged,
  Sites,
  SetSshCredential,
  Tags,
  UpdateDevice,
  type DeviceDetail,
  type DevicePage,
  type Site,
  type ReadingsChange,
  type StatusChange,
  type StoredCredential,
} from './api';

/** At most this many pages of 200 hosts are read: the hub's page limit times a bound no home lab reaches. */
const MAX_HOST_PAGES = 50;

/** The answer when the hosts cannot all be read: a partial list could miss this computer's device (R-060). */
const HOSTS_INCOMPLETE: HubError = {
  code: 'malformed',
  message: 'The hub did not list every device.',
};

/**
 * Every device's host, page by page, or an error unless the last page was reached: a page that says there is
 * more but gives no cursor, or more than `MAX_HOST_PAGES` pages, fails rather than returning part of the list.
 */
async function readHosts(hub: Hub): Promise<HubResult<readonly string[]>> {
  const hosts: string[] = [];
  let after: string | null = null;
  for (let page = 0; page < MAX_HOST_PAGES; page += 1) {
    const result: HubResult<DeviceHostsQuery> = await hub.request(DeviceHosts, { after });
    if (!result.ok) return result;
    const { edges, pageInfo }: DeviceHostsQuery['devices'] = result.value.devices;
    hosts.push(...edges.map((edge) => edge.node.host));
    if (!pageInfo.hasNextPage) return { ok: true, value: hosts };
    after = pageInfo.endCursor ?? null;
    if (after === null) return { ok: false, error: HOSTS_INCOMPLETE };
  }
  return { ok: false, error: HOSTS_INCOMPLETE };
}

/** A new site (VPN-001); the hub refuses a subnet that overlaps another site's. */
async function createSite(hub: Hub, name: string, subnet: string): Promise<HubResult<Site>> {
  const result = await hub.request(CreateSite, { name, subnet });
  return result.ok ? { ok: true, value: result.value.createSite } : result;
}

/** The stored SSH, RDP and VNC secrets: set (SSH only here; RDP and VNC are set through the sessions client) and clear. */
function credentialCalls(hub: Hub) {
  return {
    setSshCredential: async (deviceId: string, credential: SshCredentialInput): Promise<HubResult<StoredCredential>> => {
      const result = await hub.request(SetSshCredential, { deviceId, credential });
      return result.ok ? { ok: true, value: result.value.setSshCredential } : result;
    },
    clearSshCredential: async (deviceId: string): Promise<HubResult<StoredCredential>> => {
      const result = await hub.request(ClearSshCredential, { deviceId });
      return result.ok ? { ok: true, value: result.value.clearSshCredential } : result;
    },
    /** Removes the stored RDP password; resolves to the RDP settings' new `credentialStored`. */
    clearRdpCredential: async (deviceId: string): Promise<HubResult<ClearRdpCredentialMutation['clearRdpCredential']>> => {
      const result = await hub.request(ClearRdpCredential, { deviceId });
      return result.ok ? { ok: true, value: result.value.clearRdpCredential } : result;
    },
    /** Removes the stored VNC password; resolves to the VNC settings' new `credentialStored`. */
    clearVncCredential: async (deviceId: string): Promise<HubResult<ClearVncCredentialMutation['clearVncCredential']>> => {
      const result = await hub.request(ClearVncCredential, { deviceId });
      return result.ok ? { ok: true, value: result.value.clearVncCredential } : result;
    },
  };
}

/** The live updates of every device: status changes and the latest readings. Each returns the function that stops listening. */
function liveCalls(hub: Hub) {
  return {
    watchStatus: (sink: Sink<StatusChange>): (() => void) =>
      hub.subscribe(DeviceStatusChanged, {}, {
        ...sink,
        next: (value) => {
          sink.next(value.deviceStatusChanged);
        },
      }),
    /** MET-004. */
    watchReadings: (sink: Sink<ReadingsChange>): (() => void) =>
      hub.subscribe(CardReadingsAdded, {}, {
        ...sink,
        next: (value) => {
          sink.next(value.metricsAdded);
        },
      }),
  };
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
    createSite: (name: string, subnet: string): Promise<HubResult<Site>> => createSite(hub, name, subnet),
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
    ...credentialCalls(hub),
    ...liveCalls(hub),
  };
}
