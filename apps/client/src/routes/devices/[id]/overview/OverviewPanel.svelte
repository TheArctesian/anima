<script lang="ts">
  import { statusLabel, type DeviceStatus } from '$lib/devices/card/card';
  import { checkCounts, hostKeyText, uptimeReadings, type OverviewDevice } from './overview';

  interface Props {
    device: OverviewDevice & { readonly status: DeviceStatus };
    checks: readonly { readonly status: DeviceStatus }[];
  }

  const { device, checks }: Props = $props();
  const counts = $derived(checkCounts(checks));
</script>

<!-- Phase 1 has no metrics (MET-003), so the panel holds no empty chart slots (D-10). -->
<dl class="facts">
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

<style>
  .facts {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, calc(var(--space-16) * 3)), 1fr));
    gap: var(--space-4);
    margin: 0;
  }

  dt {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  dd {
    margin: var(--space-1) 0 0;
    color: var(--text);
  }

  .reading {
    font-variant-numeric: tabular-nums;
  }

  .count + .count::before {
    content: ' · ';
  }

  .key {
    overflow-wrap: anywhere;
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }
</style>
