<script lang="ts">
  import { statusLabel, type DeviceStatus } from '$lib/devices/card/card';
  import type { MetricsSource } from '$lib/hub/generated/graphql';
  import type { MetricRange } from '$lib/metrics/api';
  import type { LoadingClock } from '$lib/motion/loading/loading';
  import './geometry.css';
  import MetricsSection from './MetricsSection.svelte';
  import type { DeviceMetrics } from './metrics.svelte';
  import { checkCounts, hostKeyText, uptimeReadings, type OverviewDevice } from './overview';

  interface Props {
    device: OverviewDevice & { readonly status: DeviceStatus; readonly metricsSource: MetricsSource };
    checks: readonly { readonly status: DeviceStatus }[];
    metrics: Pick<DeviceMetrics, 'view' | 'span' | 'readings' | 'error' | 'liveError'>;
    range: MetricRange;
    now: () => Date;
    clock?: LoadingClock;
    onRange: (range: MetricRange) => void;
    onRetry: () => void;
    onSettings: () => void;
  }

  const { device, checks, metrics, range, now, clock, onRange, onRetry, onSettings }: Props = $props();
  const counts = $derived(checkCounts(checks));
</script>

<dl class="overview-facts">
  <div>
    <dt>Status</dt>
    <!-- A plain label: the header's pill is the one that moves from the card (D-12). -->
    <dd>{statusLabel(device.status)}</dd>
  </div>
  <div>
    <dt>Maintenance</dt>
    <dd>{device.maintenance ? 'On' : 'Off'}</dd>
  </div>
  {#each uptimeReadings(device) as reading (reading.label)}
    <div>
      <dt>Uptime, {reading.label}</dt>
      <dd class="reading">{reading.text}</dd>
    </div>
  {/each}
  <div>
    <dt>Checks</dt>
    <dd>
      {#each counts as entry (entry.status)}
        <span class="count">{entry.count} {entry.label}</span>
      {:else}
        No checks
      {/each}
    </dd>
  </div>
  <div>
    <dt>SSH host key</dt>
    <dd class="key">{hostKeyText(device.ssh)}</dd>
  </div>
</dl>

<MetricsSection source={device.metricsSource} {metrics} {range} {now} {clock} {onRange} {onRetry} {onSettings} />

<style>
  .reading {
    font-variant-numeric: tabular-nums;
  }

  .count + .count::before {
    content: ' · ';
  }
</style>
