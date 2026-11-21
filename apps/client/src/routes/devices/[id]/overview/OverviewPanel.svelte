<script lang="ts">
  import type { DeviceStatus } from '$lib/devices/card/card';
  import type { MetricsSource } from '$lib/hub/generated/graphql';
  import type { MetricRange } from '$lib/metrics/api';
  import type { LoadingClock } from '$lib/motion/loading/loading';
  import type { VpnNode } from '$lib/vpn/api';
  import PlanUsageCard from '$lib/hosted/usage/PlanUsageCard.svelte';
  import { readAccountUsage } from '$lib/hosted/usage/read';
  import { hubDataBytes } from '$lib/hosted/usage/usage';
  import StorageSection from '$lib/metrics/storage/StorageSection.svelte';
  import DeviceVpn from '$lib/vpn/device/DeviceVpn.svelte';
  import './geometry.css';
  import HostKeyCard from './facts/HostKeyCard.svelte';
  import OverviewFacts from './facts/OverviewFacts.svelte';
  import MetricsSection from './MetricsSection.svelte';
  import type { DeviceMetrics } from './metrics.svelte';
  import type { OverviewDevice } from './overview';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import type { WebInterface } from '$lib/web-interfaces/api';
  import type { SettingsSection } from '../tabs/tabs';
  import AboutCard from './side/AboutCard.svelte';
  import ConnectCard from './side/ConnectCard.svelte';
  import type { SideDevice } from './side/side';

  interface Props {
    device: OverviewDevice & SideDevice & { readonly status: DeviceStatus; readonly metricsSource: MetricsSource };
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
    /** The ways in of the card "Connect"; the card is not drawn where the page gives none (the phone has its own list). */
    onConnect?: ((protocol: SessionProtocol, button: HTMLElement) => void) | undefined;
    onOpenWeb?: ((webInterface: WebInterface, button: HTMLElement) => void) | undefined;
    onSection?: ((section: SettingsSection | null) => void) | undefined;
  }

  const { device, checks, metrics, range, vpn = null, now, clock, onRange, onRetry, onSettings, onConnect, onOpenWeb, onSection }: Props = $props();
</script>

<!-- The facts and the resources in the main column; beside them, how to get in and what the device is (R-224). On a
     narrow panel the two cards come first, as one row or one column. -->
<div class="overview-fit">
  <div class="overview-columns">
    <div class="overview-main">
      <OverviewFacts {device} {checks} hostKey={false} />

      <MetricsSection source={device.metricsSource} {metrics} {range} {now} {clock} {onRange} {onRetry} {onSettings} />

      <StorageSection source={device.metricsSource} drives={metrics.view?.drives ?? null} error={metrics.error} />

      <!-- After Metrics: the picture comes from its own query and may arrive late, so nothing above it moves (MOT-007). -->
      {#if vpn}
        <DeviceVpn node={vpn} now={now()} />
      {/if}
    </div>
    <aside class="overview-side" aria-label="Connect and about">
      {#if onConnect !== undefined && onSection !== undefined}
        <ConnectCard {device} {onConnect} {onOpenWeb} onSettings={onSection} />
      {/if}
      {#if device.kind === 'hub'}
        <!-- HMD-073: the page of the hub asks the account for the figures of its plan; the hub holds none. -->
        <PlanUsageCard read={() => readAccountUsage()} dataBytes={hubDataBytes(metrics.view?.series ?? [])} {now} />
      {/if}
      <HostKeyCard ssh={device.ssh} />
      <AboutCard {device} />
    </aside>
  </div>
</div>
