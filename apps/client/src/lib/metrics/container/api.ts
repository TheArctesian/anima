import { graphql } from '$lib/hub/generated';
import type { ContainerSamplesAddedSubscription } from '$lib/hub/generated/graphql';
import type { Hub, Sink } from '$lib/hub/hub';

// K1 gives the bytes of memory in use only as the series MEMORY_BYTES; the list reads them from the batches of one device.
const ContainerSamplesAdded = graphql(`
  subscription ContainerSamplesAdded($deviceId: ID!) {
    metricsAdded(deviceId: $deviceId) {
      deviceId
      samples {
        kind
        label
        value
      }
    }
  }
`);

/** The samples of one batch of the hub's own device. */
export type ContainerBatch = ContainerSamplesAddedSubscription['metricsAdded'];

/** The calls of the hub for the figures of its own container. */
export function containerApi(hub: Pick<Hub, 'subscribe'>) {
  return {
    /** Listen for the batches of device `deviceId`; returns the function that stops listening. */
    watch: (deviceId: string, sink: Sink<ContainerBatch>): (() => void) =>
      hub.subscribe(ContainerSamplesAdded, { deviceId }, {
        ...sink,
        next: (value) => {
          sink.next(value.metricsAdded);
        },
      }),
  };
}
