<script lang="ts">
  import { combinedStatus, type CheckState, type StatusFacts } from '$lib/devices/status/precedence';
  import StatusBadge from '$lib/devices/status/StatusBadge.svelte';

  const BASE: StatusFacts = { maintenance: false, checks: [], securityWarnings: [], expiries: [], warnWithinDays: 7, metricsOverThreshold: [] };
  const check = (name: string, status: CheckState['status'], paused = false): CheckState => ({ name, status, paused });

  /** One device per status; the facts are what DEV-006 reads, and the badge shows what the precedence returns. */
  const DEVICES: readonly { id: string; name: string; facts: StatusFacts }[] = [
    { id: 'maint', name: 'printer', facts: { ...BASE, maintenance: true, checks: [check('ping', 'DOWN')] } },
    { id: 'down', name: 'nas', facts: { ...BASE, checks: [check('ping', 'DOWN'), check('web', 'DEGRADED')], metricsOverThreshold: ['Disk'] } },
    { id: 'warn', name: 'router', facts: { ...BASE, checks: [check('ping', 'UP')], securityWarnings: ['HOST_KEY_CHANGED'], expiries: [{ kind: 'CERTIFICATE', name: 'router.lan', daysLeft: 5 }] } },
    { id: 'deg', name: 'proxy', facts: { ...BASE, checks: [check('ping', 'UP'), check('web', 'DEGRADED')] } },
    { id: 'up', name: 'switch', facts: { ...BASE, checks: [check('ping', 'UP')] } },
    { id: 'unknown', name: 'camera', facts: { ...BASE, checks: [check('ping', 'UP', true)] } },
  ];
  const results = DEVICES.map((device) => ({ ...device, result: combinedStatus(device.facts) }));
</script>

<div class="stack">
  <h3>Combined status (DEV-006): the badge shows the winner, the reasons are for a HelpTip</h3>
  <ul>
    {#each results as device (device.id)}
      <li>
        <span class="name">{device.name}</span>
        <span class="badge"><StatusBadge status={device.result.status} deviceId={`specimen-${device.id}`}>
          {#snippet reasons()}<span class="tip" title={device.result.reasons.map((reason) => reason.text).join('\n')}>?</span>{/snippet}
        </StatusBadge></span>
        <span class="why">{device.result.reasons.map((reason) => reason.text).join('. ')}</span>
      </li>
    {/each}
  </ul>
</div>

<style>
  .stack {
    display: grid;
    gap: var(--space-3);
  }

  h3 {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-md);
  }

  ul {
    display: grid;
    gap: var(--space-2);
    margin: 0;
    padding: var(--space-4);
    background: var(--surface-raised);
    border-radius: var(--radius-lg);
    box-shadow: var(--elevation-1);
    list-style: none;
  }

  li {
    display: grid;
    grid-template-columns: 6rem 11rem minmax(0, 1fr);
    align-items: center;
    gap: var(--space-3);
  }

  .badge {
    display: flex;
    align-items: center;
  }

  .why {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .tip {
    display: inline-grid;
    place-items: center;
    inline-size: var(--space-4);
    block-size: var(--space-4);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-pill);
    color: var(--text-secondary);
    font-size: var(--text-xs);
  }
</style>
