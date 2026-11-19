import { graphql } from '$lib/hub/generated';
import type { ContainerLogQuery, ContainersQuery } from '$lib/hub/generated/graphql';
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

export type Container = ContainersQuery['containers'][number];
export type ContainerLog = ContainerLogQuery['containerLog'];
export type LogLine = ContainerLog['lines'][number];

/** What the container screen asks of the hub; the screen and its tests take this, not the hub. */
export interface ContainerCalls {
  list(): Promise<HubResult<readonly Container[]>>;
  log(containerId: string, lines: number): Promise<HubResult<ContainerLog>>;
}

/** The container calls of the hub for one device (HOST-001). The hub reads the device at each call and keeps nothing. */
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
  };
}
