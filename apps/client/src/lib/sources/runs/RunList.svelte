<script lang="ts">
  import type { SyncRun } from '../api';
  import { runLine } from './runs';

  interface Props {
    runs: readonly SyncRun[];
    now: Date;
  }

  const { runs, now }: Props = $props();
  const lines = $derived(runs.map((run) => runLine(run, now)));
</script>

{#if lines.length === 0}
  <p class="none">No sync has run yet.</p>
{:else}
  <ul class="runs" aria-label="Recent syncs">
    {#each lines as line (line.id)}
      <li>
        <span class="when">{line.trigger}. {line.started}. {line.ended}. {line.outcome}.</span>
        <ul class="counts">
          {#each line.counts as count (count.label)}
            <li>{count.label} {count.value}</li>
          {/each}
        </ul>
        {#if line.error !== null}<span class="error">{line.error}</span>{/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  .runs,
  .counts {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .runs {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .counts {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-3);
  }

  .runs li {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    font-size: var(--text-sm);
  }

  .counts li {
    color: var(--text-secondary);
  }

  .none,
  .when {
    margin: 0;
  }

  .error {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }
</style>
