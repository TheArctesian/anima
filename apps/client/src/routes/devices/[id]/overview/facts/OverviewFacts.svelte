<script lang="ts">
  import type { DeviceStatus } from '$lib/devices/card/card';
  import Icon from '$lib/icons/Icon.svelte';
  import { checkCounts, uptimeReadings, type OverviewDevice } from '../overview';
  import HostKeyCard from './HostKeyCard.svelte';
  import Reading from './Reading.svelte';
  import StatusDot from './StatusDot.svelte';

  interface Props {
    device: OverviewDevice & { readonly status: DeviceStatus };
    checks: readonly { readonly status: DeviceStatus }[];
  }

  const { device, checks }: Props = $props();
  const id = $props.id();
  const counts = $derived(checkCounts(checks));
</script>

<!-- D-10: status, uptime, checks and the host key, each a card; the skeleton draws this same component with its text hidden (MOT-007). -->
<div class="overview-facts-fit">
  <div class="overview-facts">
    <section class="overview-card" aria-labelledby="{id}-status">
      <h2 id="{id}-status" class="overview-card-title"><Icon name="nf-md-heart_pulse" />Status</h2>
      <dl class="facts">
        <div>
          <dt>Status</dt>
          <dd><Reading value={device.status}><StatusDot status={device.status} /></Reading></dd>
        </div>
        <div>
          <dt>Maintenance</dt>
          <dd><Icon name="nf-md-wrench_outline" size="sm" /><Reading value={device.maintenance}>{device.maintenance ? 'On' : 'Off'}</Reading></dd>
        </div>
      </dl>
    </section>
    <section class="overview-card" aria-labelledby="{id}-uptime">
      <h2 id="{id}-uptime" class="overview-card-title"><Icon name="nf-md-timer_outline" />Uptime</h2>
      <dl class="overview-figures">
        {#each uptimeReadings(device) as reading (reading.label)}
          <div class="overview-figure">
            <dt>{reading.label}</dt>
            <dd><Reading value={reading.text}>{reading.text}</Reading></dd>
          </div>
        {/each}
      </dl>
    </section>
    <section class="overview-card" aria-labelledby="{id}-checks">
      <h2 id="{id}-checks" class="overview-card-title"><Icon name="nf-md-clipboard_check_outline" />Checks</h2>
      {#if counts.length > 0}
        <!-- Worst first (R-044); each count is labelled with its status in words beside its dot (UI-003). -->
        <dl class="overview-figures">
          {#each counts as entry (entry.status)}
            <div class="overview-figure">
              <dt><StatusDot status={entry.status} /></dt>
              <dd><Reading value={entry.count}>{entry.count}</Reading></dd>
            </div>
          {/each}
        </dl>
      {:else}
        <p class="overview-figures none">No checks</p>
      {/if}
    </section>
    <HostKeyCard ssh={device.ssh} />
  </div>
</div>

<style>
  .facts {
    display: grid;
    gap: var(--space-2);
    margin: 0;
  }

  .facts div {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    min-block-size: var(--target-min-desktop);
  }

  .facts dt {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .facts dd {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    color: var(--text);
  }

  .none {
    align-items: center;
    color: var(--text-muted);
  }
</style>
