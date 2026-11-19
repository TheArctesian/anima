import { graphql } from '$lib/hub/generated';
import type { ContainerDetailQuery, ContainerImageUpdatesQuery, ContainerLogQuery, ContainerNetworksQuery, ContainerStorageQuery, ContainersQuery } from '$lib/hub/generated/graphql';
import type { Hub, HubResult } from '$lib/hub/hub';

const CONTAINERS = graphql(`
  query Containers($deviceId: ID!) {
    containers(deviceId: $deviceId) {
      id
      name
      image
      state
      status
      health
      ports
      createdAt
      composeProject
      composeService
      composeProjectState
    }
  }
`);

const CONTAINER_LOG = graphql(`
  query ContainerLog($deviceId: ID!, $containerId: String!, $lines: Int) {
    containerLog(deviceId: $deviceId, containerId: $containerId, lines: $lines) {
      lines {
        at
        text
      }
      cut
    }
  }
`);

const CONTAINER_DETAIL = graphql(`
  query ContainerDetail($deviceId: ID!, $containerId: String!) {
    containerDetail(deviceId: $deviceId, containerId: $containerId) {
      id
      name
      image
      imageId
      command
      workingDir
      user
      restartPolicy
      state
      exitCode
      startedAt
      finishedAt
      restartCount
      healthCheck {
        test
        intervalSeconds
      }
      health
      failingStreak
      lastHealthRun {
        endedAt
        exitCode
      }
      mounts {
        type
        name
        source
        destination
        readOnly
      }
      networks {
        network
        ipv4
        ipv6
        gateway
        macAddress
      }
      labels {
        name
        value
      }
      environmentNames
      processes {
        pid
        user
        command
      }
    }
  }
`);

const CONTAINER_STORAGE = graphql(`
  query ContainerStorage($deviceId: ID!) {
    containerStorage(deviceId: $deviceId) {
      images {
        id
        repository
        tag
        size
        sharedSize
        uniqueSize
        created
        containerCount
        containers
      }
      volumes {
        name
        driver
        size
        containerCount
        containers
      }
      usage {
        type
        totalCount
        activeCount
        size
        reclaimable
      }
    }
  }
`);

const CONTAINER_NETWORKS = graphql(`
  query ContainerNetworks($deviceId: ID!) {
    containerNetworks(deviceId: $deviceId) {
      id
      name
      driver
      scope
      internal
      containers
    }
  }
`);

const CONTAINER_IMAGE_UPDATES = graphql(`
  query ContainerImageUpdates($deviceId: ID!) {
    containerImageUpdates(deviceId: $deviceId) {
      containerId
      container
      image
      state
      reason
      localDigest
      registryDigest
    }
  }
`);

export type Container = ContainersQuery['containers'][number];
export type ContainerLog = ContainerLogQuery['containerLog'];
export type LogLine = ContainerLog['lines'][number];
export type ContainerDetail = ContainerDetailQuery['containerDetail'];
export type ContainerStorage = ContainerStorageQuery['containerStorage'];
export type ContainerImage = ContainerStorage['images'][number];
export type ContainerVolume = ContainerStorage['volumes'][number];
export type ContainerNetwork = ContainerNetworksQuery['containerNetworks'][number];
export type ImageUpdate = ContainerImageUpdatesQuery['containerImageUpdates'][number];

/** What the container screen asks of the hub; the screen and its tests take this, not the hub. */
export interface ContainerCalls {
  list(): Promise<HubResult<readonly Container[]>>;
  log(containerId: string, lines: number): Promise<HubResult<ContainerLog>>;
  /** HOST-017: the details of one container, with the names of its environment variables and no value. */
  detail(containerId: string): Promise<HubResult<ContainerDetail>>;
  /** HOST-020: the images and the volumes, with the space each type uses. */
  storage(): Promise<HubResult<ContainerStorage>>;
  networks(): Promise<HubResult<readonly ContainerNetwork[]>>;
  /** HOST-021: whether a newer image exists for each container. The hub asks public registries; it updates nothing. */
  updates(): Promise<HubResult<readonly ImageUpdate[]>>;
}

/** The container calls of the hub for one device (HOST-001, HOST-017 to HOST-021). The hub reads the device at each call and keeps nothing. */
export function containersApi(hub: Pick<Hub, 'request'>, deviceId: string): ContainerCalls {
  return {
    list: async () => {
      const result = await hub.request(CONTAINERS, { deviceId });
      return result.ok ? { ok: true, value: result.value.containers } : result;
    },
    log: async (containerId, lines) => {
      const result = await hub.request(CONTAINER_LOG, { deviceId, containerId, lines });
      return result.ok ? { ok: true, value: result.value.containerLog } : result;
    },
    detail: async (containerId) => {
      const result = await hub.request(CONTAINER_DETAIL, { deviceId, containerId });
      return result.ok ? { ok: true, value: result.value.containerDetail } : result;
    },
    storage: async () => {
      const result = await hub.request(CONTAINER_STORAGE, { deviceId });
      return result.ok ? { ok: true, value: result.value.containerStorage } : result;
    },
    networks: async () => {
      const result = await hub.request(CONTAINER_NETWORKS, { deviceId });
      return result.ok ? { ok: true, value: result.value.containerNetworks } : result;
    },
    updates: async () => {
      const result = await hub.request(CONTAINER_IMAGE_UPDATES, { deviceId });
      return result.ok ? { ok: true, value: result.value.containerImageUpdates } : result;
    },
  };
}
