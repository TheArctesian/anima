<script lang="ts">
  import { countText, historyText, type HubKept } from './kept';

  interface Props {
    /** Asks the hub for its counts and its retention periods (`read.ts`). */
    read: () => Promise<HubKept>;
  }

  const { read }: Props = $props();
  let kept = $state<HubKept | null>(null);

  $effect(() => {
    let current = true;
    void read()
      .catch((): HubKept => ({ devices: null, checks: null, periods: null }))
      .then((value) => {
        if (current) kept = value;
      });
    return () => {
      current = false;
    };
  });
</script>

<!-- What the hub keeps (HMD-073, task HM-T36 of hosted-mode.md): the figures of the hub itself, beside the figures of the
     account in "Plan and usage". -->
<section class="overview-card hub-kept" aria-labelledby="overview-hub-kept" aria-busy={kept === null}>
  <h2 id="overview-hub-kept" class="overview-card-title">What the hub keeps</h2>
  <dl>
    <div><dt>Devices</dt><dd class:quiet={kept === null}>{kept === null ? 'Loading…' : countText(kept.devices)}</dd></div>
    <div><dt>Checks</dt><dd class:quiet={kept === null}>{kept === null ? 'Loading…' : countText(kept.checks)}</dd></div>
    <div><dt>History</dt><dd class:quiet={kept === null}>{kept === null ? 'Loading…' : historyText(kept.periods)}</dd></div>
  </dl>
</section>

<style>
  dl {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
    font-size: var(--text-sm);
  }

  dl > div {
    display: grid;
    grid-template-columns: calc(var(--space-16) + var(--space-8)) minmax(0, 1fr);
    gap: var(--space-3);
  }

  dt {
    color: var(--text-secondary);
  }

  dd {
    margin: 0;
    color: var(--text);
    font-variant-numeric: tabular-nums;
    overflow-wrap: anywhere;
  }

  .quiet {
    color: var(--text-tertiary);
  }
</style>
