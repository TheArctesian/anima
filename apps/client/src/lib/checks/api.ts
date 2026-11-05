import { graphql } from '$lib/hub/generated';
import type { HeartbeatAddedSubscription } from '$lib/hub/generated/graphql';
import type { Hub, Sink } from '$lib/hub/hub';

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

/** A heartbeat the hub just recorded, with the check and device it belongs to. */
export type NewHeartbeat = HeartbeatAddedSubscription['heartbeatAdded'];

/** Live heartbeats through a `Hub` (CQ-045). */
export function checksApi(hub: Hub) {
  return {
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
