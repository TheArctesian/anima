import { graphql } from '$lib/hub/generated';
import type { DevicesQuery, DevicesQueryVariables } from '$lib/hub/generated/graphql';
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

/** One page of the device list, as the hub answers it. */
export type DevicePage = DevicesQuery['devices'];

/** A device with every field a card or table row shows. */
export type CardDevice = DevicePage['edges'][number]['node'];

/** A device's new status, as the hub pushes it (DEV-003). */
export interface StatusChange {
  readonly deviceId: string;
  readonly status: CardDevice['status'];
  readonly at: string;
}

/** The device list, tags and live status changes through a `Hub` (CQ-045). */
export function devicesApi(hub: Hub) {
  return {
    list: async (variables: DevicesQueryVariables): Promise<HubResult<DevicePage>> => {
      const result = await hub.request(Devices, variables);
      return result.ok ? { ok: true, value: result.value.devices } : result;
    },
    tags: async (): Promise<HubResult<readonly string[]>> => {
      const result = await hub.request(Tags, {});
      return result.ok ? { ok: true, value: result.value.tags } : result;
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
