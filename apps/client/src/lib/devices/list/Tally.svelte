<script lang="ts">
  import type { Status } from '$lib/hub/generated/graphql';
  import type { ListQuery } from '../query';
  import { tallyQuery, type Tally } from './look';

  interface Props {
    /** `null` until the first list has come: the line then has its loading form and never says "0 devices". */
    tally: Tally | null;
    /** Whether a list can still come; without it the line is empty and keeps its height. */
    waiting?: boolean;
    query: ListQuery;
    onQueryChange: (query: ListQuery) => void;
  }

  const { tally, waiting = true, query, onQueryChange }: Props = $props();
  const only = $derived(query.statuses.length === 1 ? query.statuses[0] : null);
  const DOTS: Record<Status, string> = { UP: 'up', DOWN: 'down', DEGRADED: 'degraded', MAINTENANCE: 'maintenance', UNKNOWN: 'unknown' };
</script>

<!-- One count line that is also the status filter (UR-7, D18): a count shows only its devices, and again shows all. -->
{#if tally === null}
  <!-- The same height as the counts, so the line does not move when they come (MOT-007). -->
  <div class="tally" aria-busy={waiting}>
    <!-- A count that is not shown gives the line the height of the counts. -->
    <button type="button" class="size" tabindex="-1" disabled aria-hidden="true"><b>0</b>devices</button>
    {#if waiting}<span class="bone" aria-hidden="true"></span><span class="hidden" role="status">Loading the devices</span>{/if}
  </div>
{:else}
<div class="tally" role="group" aria-label="Show devices by status">
  <button type="button" tabindex="0" aria-pressed={query.statuses.length === 0} onclick={() => { onQueryChange(tallyQuery(query, null)); }}>
    <b>{tally.total}</b>{tally.total === 1 ? 'device' : 'devices'}
  </button>
  {#each tally.parts as part (part.status)}
    <button type="button" tabindex="0" aria-pressed={only === part.status} onclick={() => { onQueryChange(tallyQuery(query, part.status)); }}>
      <span class="dot {DOTS[part.status]}" aria-hidden="true"></span><b>{part.count}</b>{part.words}
    </button>
  {/each}
</div>
{/if}

<style>
  .tally {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-5);
  }

  .tally:not([role]) {
    position: relative;
    align-items: center;
  }

  .size {
    visibility: hidden;
  }

  /* A quiet block where the total will stand. */
  .bone {
    position: absolute;
    inset-inline-start: 0;
    inline-size: calc(var(--space-16) * 2);
    block-size: var(--text-lg);
    border-radius: var(--radius-sm);
    background: var(--surface-raised);
  }

  .hidden {
    position: absolute;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  button {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    block-size: var(--control-md);
    padding: 0;
    border: 0;
    border-block-end: var(--border-thin) solid transparent;
    background: none;
    color: var(--text-tertiary);
    font: inherit;
    font-size: var(--text-lg);
    cursor: pointer;
    transition: color var(--duration-fast) var(--ease-enter), border-color var(--duration-fast) var(--ease-enter);
  }

  b {
    color: var(--text);
    font-weight: var(--weight-medium);
    font-variant-numeric: tabular-nums;
  }

  button:hover {
    color: var(--text-secondary);
  }

  button[aria-pressed='true'] {
    border-block-end-color: var(--accent);
    color: var(--text-secondary);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
    border-radius: var(--radius-sm);
  }

  .dot {
    inline-size: var(--space-2);
    block-size: var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--status-unknown);
  }

  .up {
    background: var(--status-up);
  }

  .down {
    background: var(--status-down);
  }

  .degraded {
    background: var(--status-degraded);
  }

  .maintenance {
    background: var(--status-maintenance);
  }

  @media (max-width: 767.98px) {
    .tally {
      gap: var(--border-focus) var(--space-4);
    }

    button {
      font-size: var(--text-md);
    }
  }
</style>
