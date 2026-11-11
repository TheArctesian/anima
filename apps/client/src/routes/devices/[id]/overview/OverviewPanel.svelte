<script lang="ts">
  import type { DeviceStatus } from '$lib/devices/card/card';
  import type { MetricsSource } from '$lib/hub/generated/graphql';
  import type { MetricRange } from '$lib/metrics/api';
  import type { LoadingClock } from '$lib/motion/loading/loading';
  import './geometry.css';
  import OverviewFacts from './facts/OverviewFacts.svelte';
  import MetricsSection from './MetricsSection.svelte';
  import type { DeviceMetrics } from './metrics.svelte';
  import type { OverviewDevice } from './overview';

  interface Props {
    device: OverviewDevice & { readonly status: DeviceStatus; readonly metricsSource: MetricsSource };
    checks: readonly { readonly status: DeviceStatus }[];
    metrics: Pick<DeviceMetrics, 'view' | 'span' | 'shownRange' | 'chartShape' | 'readings' | 'error' | 'liveError'>;
    range: MetricRange;
    now: () => Date;
    clock?: LoadingClock;
    onRange: (range: MetricRange) => void;
    onRetry: () => void;
    onSettings: () => void;
  }

  const { device, checks, metrics, range, now, clock, onRange, onRetry, onSettings }: Props = $props();
</script>

<OverviewFacts {device} {checks} />

<MetricsSection source={device.metricsSource} {metrics} {range} {now} {clock} {onRange} {onRetry} {onSettings} />
