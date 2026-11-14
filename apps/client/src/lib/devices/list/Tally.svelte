<script lang="ts">
  import type { Status } from '$lib/hub/generated/graphql';
  import type { ListQuery } from '../query';
  import { tallyQuery, type Tally } from './look';

  interface Props {
    tally: Tally;
    query: ListQuery;
    onQueryChange: (query: ListQuery) => void;
  }

  const { tally, query, onQueryChange }: Props = $props();
  const only = $derived(query.statuses.length === 1 ? query.statuses[0] : null);
  const DOTS: Record<Status, string> = { UP: 'up', DOWN: 'down', DEGRADED: 'degraded', MAINTENANCE: 'maintenance', UNKNOWN: 'unknown' };
</script>

<!-- One count line that is also the status filter (UR-7, D18): a count shows only its devices, and again shows all. -->
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

<style>
  .tally {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-5);
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
