import { graphql } from '$lib/hub/generated';
import type { BatteryChargesAddedSubscription } from '$lib/hub/generated/graphql';
import type { Hub, Sink } from '$lib/hub/hub';

// The hub keeps the charge only as the series BATTERY_PERCENT (MET-032); a list reads it from the batches as they come.
const BatteryChargesAdded = graphql(`
  subscription BatteryChargesAdded {
    metricsAdded(deviceId: null) {
      deviceId
      samples {
        kind
        label
        value
      }
    }
  }
`);

/** The samples of one batch of one device; only those of kind BATTERY_PERCENT are charges. */
export type ChargeBatch = BatteryChargesAddedSubscription['metricsAdded'];

/** The battery calls of the hub. */
export function batteryApi(hub: Pick<Hub, 'subscribe'>) {
  return {
    /** Listen for the batches of every device; returns the function that stops listening. */
    watch: (sink: Sink<ChargeBatch>): (() => void) =>
      hub.subscribe(BatteryChargesAdded, {}, {
        ...sink,
        next: (value) => {
          sink.next(value.metricsAdded);
        },
      }),
  };
}
