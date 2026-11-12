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
        diskMount
        otherDrives
        vramPercent
        vramGpu {
          index
          name
          vendor
          hasVram
        }
      }
      storage {
        drives {
          name
          kind
          sizeBytes
          mounted
          mounts {
            path
            usedBytes
            sizeBytes
            percent
          }
          readBps
          writeBps
          busyPercent
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
        diskMount
        otherDrives
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

const AgentStatus = graphql(`
  query AgentStatus($id: ID!) {
    device(id: $id) {
      id
      metricsSource
      metricsStale
    }
  }
`);

const EnrolAgent = graphql(`
  mutation EnrolAgent($deviceId: ID!) {
    enrolAgent(deviceId: $deviceId) {
      code
      expiresAt
    }
  }
`);

const RevokeAgentToken = graphql(`
  mutation RevokeAgentToken($deviceId: ID!) {
    revokeAgentToken(deviceId: $deviceId) {
      id
      metricsSource
      metricsStale
    }
  }
`);

type DeviceNode = NonNullable<DeviceMetricsQuery['device']>;

/** The metric series of one device for one range (WP-28 D-5: HOUR minutes, DAY ten minutes, WEEK hours). */
export type MetricSeries = DeviceNode['metrics'][number];
export type MetricKind = MetricSeries['kind'];
export type DeviceGpu = DeviceNode['gpus'][number];
/** One Storage row: a drive, or a mount on no drive the hub knows (MET-024, MET-026). */
export type StorageDrive = DeviceNode['storage']['drives'][number];
/** One batch of new samples the hub pushes for a device (D-5). */
export type NewMetrics = MetricsAddedSubscription['metricsAdded'];
/** What the Overview needs from one metrics request. */
export interface DeviceMetricsView {
  readonly metricsSource: DeviceNode['metricsSource'];
  readonly gpus: readonly DeviceGpu[];
  readonly series: readonly MetricSeries[];
  /** The device's storage drive by drive, as the hub ordered it (MET-021). */
  readonly drives: readonly StorageDrive[];
  /** The latest readings the hub holds, for the bars before the first live batch; `null` when it has none. */
  readonly readings: DeviceNode['readings'] | null;
}
/** The agent state of one device: its source, and whether an AGENT device has gone quiet (MET-018). */
export interface AgentStatusView {
  readonly metricsSource: DeviceNode['metricsSource'];
  readonly metricsStale: boolean;
}
/** A one-use enrolment code and when it stops working (MET-012). Show it once; never log or store it (LOG-007). */
export interface AgentEnrolment {
  readonly code: string;
  readonly expiresAt: string;
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
      const view = { metricsSource: device.metricsSource, gpus: device.gpus, series: device.metrics, drives: device.storage.drives };
      return { ok: true, value: { ...view, readings: device.readings ?? null } };
    },
    /** The device's agent source and stale flag; `null` when the hub has no such device. */
    agentStatus: async (deviceId: string): Promise<HubResult<AgentStatusView | null>> => {
      const result = await hub.request(AgentStatus, { id: deviceId });
      if (!result.ok) return result;
      const { device } = result.value;
      return { ok: true, value: device ? { metricsSource: device.metricsSource, metricsStale: device.metricsStale } : null };
    },
    /** Makes a fresh enrolment code for the device (MET-012); a new code voids the earlier unused ones (K8 Section 3) and each works once. */
    enrolAgent: async (deviceId: string): Promise<HubResult<AgentEnrolment>> => {
      const result = await hub.request(EnrolAgent, { deviceId });
      return result.ok ? { ok: true, value: result.value.enrolAgent } : result;
    },
    /** Revokes the device's agent token (MET-013); the source stays AGENT, shown stale (R-092 D-7). */
    revokeAgentToken: async (deviceId: string): Promise<HubResult<AgentStatusView>> => {
      const result = await hub.request(RevokeAgentToken, { deviceId });
      if (!result.ok) return result;
      const { metricsSource, metricsStale } = result.value.revokeAgentToken;
      return { ok: true, value: { metricsSource, metricsStale } };
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
