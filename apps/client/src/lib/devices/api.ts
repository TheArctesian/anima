import { graphql } from '$lib/hub/generated';
import type {
  AccessWarning,
  CardReadingsFragment,
  DeviceQuery,
  DevicesQuery,
  SetSshCredentialMutation,
  SitesQuery,
} from '$lib/hub/generated/graphql';

export const Devices = graphql(`
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
          os
          distribution
          macAddress
          metricsSource
          readings {
            ...CardReadings
          }
          accessWarnings
          webInterfaces {
            ...WebInterfaceFields
          }
          rdp {
            enabled
          }
          vnc {
            enabled
            username
          }
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

export const DeviceHosts = graphql(`
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

export const Tags = graphql(`
  query Tags {
    tags
  }
`);

export const DeviceStatusChanged = graphql(`
  subscription DeviceStatusChanged {
    deviceStatusChanged(deviceId: null) {
      deviceId
      status
      at
    }
  }
`);

export const CardReadingsAdded = graphql(`
  subscription CardReadingsAdded {
    metricsAdded(deviceId: null) {
      deviceId
      readings {
        ...CardReadings
      }
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
    distribution
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
    macAddress
    metricsSource
    accessWarnings
    rdp {
      enabled
      port
      username
      domain
      security
      ignoreCertificate
      resizeMethod
      audioOut
      microphone
      sharedDrive
      credentialStored
      serverCertificate {
        fingerprintSha256
        subject
        notAfter
        firstSeenAt
      }
    }
    vnc {
      enabled
      port
      username
      credentialStored
    }
    transfer {
      upload
      download
      copy
      paste
    }
    webInterfaces {
      ...WebInterfaceFields
    }
    webInterfaceSuggestions {
      ...WebInterfaceSuggestionFields
    }
    day: uptime(window: DAY)
    week: uptime(window: WEEK)
    month: uptime(window: MONTH)
  }
`);

// The latest readings a card shows; `Devices` and `CardReadingsAdded` spread it (MET-004). Codegen inlines it.
void graphql(`
  fragment CardReadings on Readings {
    at
    cpuPercent
    memoryPercent
    diskPercent
    vramPercent
    vramGpu {
      index
      name
      vendor
      hasVram
    }
  }
`);

export const Device = graphql(`
  query Device($id: ID!) {
    device(id: $id) {
      ...DeviceDetailFields
    }
  }
`);

export const Sites = graphql(`
  query Sites {
    sites {
      id
      name
      subnet
    }
  }
`);

export const CreateSite = graphql(`
  mutation CreateSite($name: String!, $subnet: Cidr!) {
    createSite(name: $name, subnet: $subnet) {
      id
      name
      subnet
    }
  }
`);

export const CreateDevice = graphql(`
  mutation CreateDevice($input: CreateDeviceInput!) {
    createDevice(input: $input) {
      ...DeviceDetailFields
    }
  }
`);

export const UpdateDevice = graphql(`
  mutation UpdateDevice($id: ID!, $input: UpdateDeviceInput!) {
    updateDevice(id: $id, input: $input) {
      ...DeviceDetailFields
    }
  }
`);

export const DeleteDevice = graphql(`
  mutation DeleteDevice($id: ID!) {
    deleteDevice(id: $id)
  }
`);

export const SetSshCredential = graphql(`
  mutation SetSshCredential($deviceId: ID!, $credential: SshCredentialInput!) {
    setSshCredential(deviceId: $deviceId, credential: $credential) {
      credentialStored
    }
  }
`);

export const ClearSshCredential = graphql(`
  mutation ClearSshCredential($deviceId: ID!) {
    clearSshCredential(deviceId: $deviceId) {
      credentialStored
    }
  }
`);

export const ClearRdpCredential = graphql(`
  mutation ClearRdpCredential($deviceId: ID!) {
    clearRdpCredential(deviceId: $deviceId) {
      credentialStored
    }
  }
`);

export const ClearVncCredential = graphql(`
  mutation ClearVncCredential($deviceId: ID!) {
    clearVncCredential(deviceId: $deviceId) {
      credentialStored
    }
  }
`);

/** One page of the device list, as the hub answers it. */
export type DevicePage = DevicesQuery['devices'];

/** The latest metric readings of a device card (MET-004). */
export type Readings = CardReadingsFragment;

/** RA-020. */
export type { AccessWarning };

/** A device with every field a card or table row shows. */
export type CardDevice = DevicePage['edges'][number]['node'];

/** One device with everything the detail page and the device form show (DEV-020, D-10, SEC-007). */
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

/** A device's new readings, as the hub pushes them (MET-004). */
export interface ReadingsChange {
  readonly deviceId: string;
  readonly readings: Readings;
}

// F-3: the pending session policy fields (RA-036, RA-040) have their own documents, so a hub without them
// still answers every other device operation.
export const DevicePolicy = graphql(`
  query DevicePolicy($id: ID!) {
    device(id: $id) {
      sessionLimit
      recordSessions
    }
  }
`);

export const SetDevicePolicy = graphql(`
  mutation SetDevicePolicy($id: ID!, $input: UpdateDeviceInput!) {
    updateDevice(id: $id, input: $input) {
      sessionLimit
      recordSessions
    }
  }
`);
