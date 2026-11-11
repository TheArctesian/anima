<script lang="ts">
  import type { DeviceStatus } from '$lib/devices/card/card';
  import type { MetricsSource } from '$lib/hub/generated/graphql';
  import type { MetricRange } from '$lib/metrics/api';
  import type { LoadingClock } from '$lib/motion/loading/loading';
  import type { VpnNode } from '$lib/vpn/api';
  import DeviceVpn from '$lib/vpn/device/DeviceVpn.svelte';
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
    /** The device's node on the VPN, when it has one (VPN-005). */
    vpn?: VpnNode | null;
    now: () => Date;
    clock?: LoadingClock;
    onRange: (range: MetricRange) => void;
    onRetry: () => void;
    onSettings: () => void;
  }

  const { device, checks, metrics, range, vpn = null, now, clock, onRange, onRetry, onSettings }: Props = $props();
</script>

<OverviewFacts {device} {checks} />

<MetricsSection source={device.metricsSource} {metrics} {range} {now} {clock} {onRange} {onRetry} {onSettings} />

<!-- After Metrics: the picture comes from its own query and may arrive late, so nothing above it moves (MOT-007). -->
{#if vpn}
  <DeviceVpn node={vpn} now={now()} />
{/if}
