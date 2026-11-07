import { graphql } from '$lib/hub/generated';
import type { DeviceMetricsQuery, MetricRange, MetricsAddedSubscription } from '$lib/hub/generated/graphql';
import type { Hub, HubResult, Sink } from '$lib/hub/hub';

const DeviceMetrics = graphql(`
  query DeviceMetrics($id: ID!, $range: MetricRange!) {
    device(id: $id) {
      id
      metricsSource
      gpus {
        index
        name
        vendor
        hasVram
      }
      readings {
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
      metrics(range: $range) {
        kind
        label
        gpuIndex
        points {
          at
          value
          min
          max
        }
      }
    }
  }
`);

const MetricsAdded = graphql(`
  subscription MetricsAdded($deviceId: ID!) {
    metricsAdded(deviceId: $deviceId) {
      deviceId
      at
      samples {
        kind
        label
        value
      }
      readings {
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
    }
  }
`);

type DeviceNode = NonNullable<DeviceMetricsQuery['device']>;

/** The metric series of one device for one range (WP-28 D-5: HOUR minutes, DAY ten minutes, WEEK hours). */
export type MetricSeries = DeviceNode['metrics'][number];
export type MetricKind = MetricSeries['kind'];
export type DeviceGpu = DeviceNode['gpus'][number];
/** One batch of new samples the hub pushes for a device (D-5). */
export type NewMetrics = MetricsAddedSubscription['metricsAdded'];
/** What the Overview needs from one metrics request. */
export interface DeviceMetricsView {
  readonly metricsSource: DeviceNode['metricsSource'];
  readonly gpus: readonly DeviceGpu[];
  readonly series: readonly MetricSeries[];
  /** The latest readings the hub holds, for the bars before the first live batch; `null` when it has none. */
  readonly readings: DeviceNode['readings'] | null;
}
export type { MetricRange };

/** Metric history and live samples through a `Hub` (CQ-045). */
export function metricsApi(hub: Hub) {
  return {
    /** The device's metrics for `range`; `null` when the hub has no such device. */
    series: async (deviceId: string, range: MetricRange): Promise<HubResult<DeviceMetricsView | null>> => {
      const result = await hub.request(DeviceMetrics, { id: deviceId, range });
      if (!result.ok) return result;
      const { device } = result.value;
      if (!device) return { ok: true, value: null };
      return { ok: true, value: { metricsSource: device.metricsSource, gpus: device.gpus, series: device.metrics, readings: device.readings ?? null } };
    },
    /** Listen for new samples of one device; returns the function that stops listening. */
    watch: (deviceId: string, sink: Sink<NewMetrics>): (() => void) =>
      hub.subscribe(
        MetricsAdded,
        { deviceId },
        {
          next: (value) => {
            sink.next(value.metricsAdded);
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
