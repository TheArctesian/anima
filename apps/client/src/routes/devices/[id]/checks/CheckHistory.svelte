<script lang="ts">
  import HeartbeatBar from '$lib/checks/HeartbeatBar.svelte';
  import { BAR_LENGTH, type Beat } from '$lib/checks/heartbeats';
  import { statusLabel } from '$lib/devices/card/card';
  import { timeLabel } from '$lib/events/panel/view';
  import { press } from '$lib/motion/roles/press';
  import type { HistoryBeat } from '../detail.svelte';
  import type { Pages } from '../pages.svelte';
  import ResponseChart from './ResponseChart.svelte';
  import { msText } from './rows';

  interface Props {
    checkId: string;
    name: string;
    pages: Pages<HistoryBeat>;
    now: () => Date;
    onClose: () => void;
  }

  const { checkId, name, pages, now, onClose }: Props = $props();
  const beats = $derived(pages.items ?? []);
  /** The bar reads oldest first, one cell per beat (New heartbeat role). */
  const bar = $derived<readonly Beat[]>(
    beats
      .slice(0, BAR_LENGTH)
      .toReversed()
      .map((beat) => ({ checkId, ...beat })),
  );
</script>

<section class="history" aria-label="Heartbeat history of {name}">
  <header>
    <h3>{name}: heartbeats</h3>
    <button type="button" onclick={onClose} {@attach press()}>Close</button>
  </header>
  {#if pages.error !== null}
    <div class="notice" role="alert">
      <p>{pages.error.message}</p>
      <button type="button" onclick={() => void pages.load()} {@attach press()}>Retry</button>
    </div>
  {:else if pages.items !== null}
    <HeartbeatBar beats={bar} />
    <ResponseChart {beats} />
    <ol class="beats">
      {#each beats as beat, index (index)}
        <li>
          <time datetime={beat.at}>{timeLabel(beat.at, now())}</time>
          <span>{statusLabel(beat.status)}</span>
          <span class="ms">{msText(beat.responseTimeMs)}</span>
          <!-- The hub's message as text only (MOT-010: not animated). -->
          <span class="message">{beat.message ?? ''}</span>
        </li>
      {:else}
        <li class="none">No heartbeats yet</li>
      {/each}
    </ol>
    {#if pages.hasOlder}
      {#if pages.olderError !== null}<p role="alert">{pages.olderError.message}</p>{/if}
      <button type="button" aria-busy={pages.loadingOlder} onclick={() => void pages.loadOlder()} {@attach press()}>Load older</button>
    {/if}
  {/if}
</section>

<style>
  .history {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-4);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  h3 {
    margin: 0;
    font-size: var(--text-md);
  }

  .beats {
    margin: 0;
    padding: 0;
    list-style: none;
    max-block-size: calc(var(--space-16) * 5);
    overflow-y: auto;
  }

  .beats li {
    display: grid;
    grid-template-columns: calc(var(--space-16) * 2) calc(var(--space-16) * 1.5) calc(var(--space-16) * 1.5) minmax(0, 1fr);
    gap: var(--space-2);
    padding-block: var(--space-1);
    border-block-end: var(--border-thin) solid var(--border);
    font-size: var(--text-sm);
  }

  .ms {
    font-variant-numeric: tabular-nums;
  }

  .message {
    overflow-wrap: anywhere;
    color: var(--text-muted);
  }

  .notice p {
    margin: 0;
  }

  button {
    align-self: flex-start;
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
