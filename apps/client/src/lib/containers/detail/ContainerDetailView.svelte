<script lang="ts">
  import type { ContainerDetail } from '../api';
  import { healthRows, mountLine, networkLine, summaryRows } from './detail';

  /**
   * HOST-017: the details of one container, read-only. Every text is the device's own and is drawn as text. The hub
   * gives the names of the environment variables and never a value.
   */
  const { detail }: { detail: ContainerDetail } = $props();

  const health = $derived(healthRows(detail));
</script>

<p class="note">A command, a health-check test, a label and a process line are shown as Docker has them, and can hold a secret. Anima keeps none of this.</p>
<div class="detail">
  <section aria-label="Container">
    <h4>Container</h4>
    <dl>
      {#each summaryRows(detail) as row (row.label)}
        <dt>{row.label}</dt>
        <dd class:mono={row.mono}>{row.value}</dd>
      {/each}
    </dl>
  </section>
  <section aria-label="Health check">
    <h4>Health check</h4>
    {#if health.length === 0}
      <p class="none">The container has no health check.</p>
    {:else}
      <dl>
        {#each health as row (row.label)}
          <dt>{row.label}</dt>
          <dd class:mono={row.mono}>{row.value}</dd>
        {/each}
      </dl>
    {/if}
  </section>
  <section aria-label="Mounts">
    <h4>Mounts</h4>
    {#if detail.mounts.length === 0}
      <p class="none">No mount.</p>
    {:else}
      <ul class="mono">{#each detail.mounts as mount, index (index)}<li>{mountLine(mount)}</li>{/each}</ul>
    {/if}
  </section>
  <section aria-label="Networks">
    <h4>Networks</h4>
    {#if detail.networks.length === 0}
      <p class="none">No network.</p>
    {:else}
      <ul class="mono">{#each detail.networks as address (address.network)}<li>{networkLine(address)}</li>{/each}</ul>
    {/if}
  </section>
  <section aria-label="Environment">
    <h4>Environment</h4>
    <p class="none">Names only. Anima never reads the value of an environment variable: it can hold a secret.</p>
    {#if detail.environmentNames.length > 0}
      <ul class="mono names">{#each detail.environmentNames as name, index (index)}<li>{name}</li>{/each}</ul>
    {/if}
  </section>
  <section aria-label="Labels">
    <h4>Labels</h4>
    {#if detail.labels.length === 0}
      <p class="none">No label.</p>
    {:else}
      <dl>
        {#each detail.labels as label (label.name)}
          <dt class="mono">{label.name}</dt>
          <dd class="mono">{label.value}</dd>
        {/each}
      </dl>
    {/if}
  </section>
  <section aria-label="Processes">
    <h4>Processes</h4>
    {#if detail.processes.length === 0}
      <p class="none">The container does not run, so it has no process.</p>
    {:else}
      <ul class="mono">{#each detail.processes as process (process.pid)}<li>{process.pid}  {process.user ?? ''}  {process.command}</li>{/each}</ul>
    {/if}
  </section>
</div>

<style>
  .detail {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(22rem, 1fr));
    gap: var(--space-4);
    padding: var(--space-3);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    color: var(--text);
    font-size: var(--text-sm);
  }

  section {
    min-inline-size: 0;
  }

  h4 {
    margin: 0 0 var(--space-2);
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  dl {
    display: grid;
    grid-template-columns: minmax(8rem, max-content) minmax(0, 1fr);
    gap: var(--space-1) var(--space-3);
    margin: 0;
  }

  dt {
    color: var(--text-secondary);
    overflow-wrap: anywhere;
  }

  dd {
    margin: 0;
    overflow-wrap: anywhere;
  }

  ul {
    margin: 0;
    padding: 0;
    list-style: none;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .names {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-3);
    margin-block-start: var(--space-2);
  }

  .mono {
    font-family: var(--font-mono);
  }

  .none,
  .note {
    margin: 0;
    color: var(--text-secondary);
  }

  .note {
    font-size: var(--text-sm);
  }
</style>
