<script lang="ts">
  import { press } from '$lib/motion/roles/press';
  import type { Pages } from '../pages.svelte';
  import { sessionRows, type DeviceSession } from './sessions';

  interface Props {
    pages: Pages<DeviceSession>;
    now: () => Date;
  }

  const { pages, now }: Props = $props();
  const rows = $derived(sessionRows(pages.items ?? [], now()));
</script>

<table class="sessions">
  <caption>Sessions on this device, newest first</caption>
  <thead>
    <tr><th scope="col">Start</th><th scope="col">End</th><th scope="col">Duration</th><th scope="col">Protocol</th><th scope="col">Operator</th><th scope="col">End reason</th></tr>
  </thead>
  <tbody>
    {#each rows as row (row.id)}
      <tr><td>{row.start}</td><td>{row.end}</td><td class="duration">{row.duration}</td><td>{row.protocol}</td><td>{row.operator}</td><td>{row.reason}</td></tr>
    {:else}
      <tr><td colspan="6" class="none">No sessions yet</td></tr>
    {/each}
  </tbody>
</table>
{#if pages.hasOlder}
  {#if pages.olderError !== null}<p role="alert">{pages.olderError.message}</p>{/if}
  <button type="button" aria-busy={pages.loadingOlder} onclick={() => void pages.loadOlder()} {@attach press()}>Load older</button>
{/if}

<style>
  .sessions {
    inline-size: 100%;
    border-collapse: collapse;
    font-size: var(--text-sm);
  }

  caption {
    padding-block-end: var(--space-2);
    color: var(--text-secondary);
    text-align: start;
  }

  th,
  td {
    padding: var(--space-2);
    border-block-end: var(--border-thin) solid var(--border-strong);
    text-align: start;
  }

  th {
    color: var(--text-secondary);
    font-weight: var(--weight-regular);
  }

  .duration {
    white-space: nowrap;
  }

  .none {
    color: var(--text-secondary);
  }

  button {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    margin-block-start: var(--space-3);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-overlay);
    color: var(--text);
    font: inherit;
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
