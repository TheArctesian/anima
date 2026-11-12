<script lang="ts">
  import LiveNotice from '$lib/hub/notice/LiveNotice.svelte';
  import { skeletonDelay } from '$lib/motion/loading/delay.svelte';
  import type { LoadingClock } from '$lib/motion/loading/loading';
  import { press } from '$lib/motion/roles/press';
  import { crossFade } from '$lib/motion/roles/transitions';
  import type { LogFeed } from './feed.svelte';
  import { newRowMotion } from './rows.svelte';
  import { SEVERITY_CHOICES, SOURCE_CHOICES, deviceLabel, labelOf, timeLabel } from './view';

  interface Props {
    feed: LogFeed;
    now: () => Date;
    /** The clock of the loading thresholds; tests pass their own. */
    clock?: LoadingClock;
  }

  const { feed, now, clock }: Props = $props();
  const rows = $derived(feed.log?.rows ?? []);
  const loading = $derived(feed.log === null && feed.error === null);
  // The clock is fixed for the life of the list.
  // svelte-ignore state_referenced_locally
  const skeleton = skeletonDelay(() => loading, clock);

  /** Placeholder rows that fill the panel's first screen. */
  const PLACES = Array.from({ length: 6 }, (_, index) => index);
  const slide = newRowMotion(() => feed.fresh);
</script>

{#if feed.error !== null}
  <div class="notice" role="alert">
    <p>{feed.error.message}</p>
    <button
      type="button"
      tabindex="0"
      onclick={() => {
        void feed.retry();
      }}
      {@attach press()}>Retry</button
    >
  </div>
{/if}
<LiveNotice message={feed.liveError?.message ?? null} subject="logs" gap="padded" />

<!-- The skeleton lies over the rows in one cell and fades off them, so the swap moves nothing (MOT-007). -->
<div class="stack">
  <ol class="rows" aria-label="Log events" aria-busy={loading}>
    {#each rows as row (row.id)}
      <li class="row" {@attach slide(row.id)}>
        <span class="glow" data-glow aria-hidden="true"></span>
        <time datetime={row.at}>{timeLabel(row.at, now())}</time>
        <span class="severity {row.severity.toLowerCase()}">{labelOf(SEVERITY_CHOICES, row.severity)}</span>
        <span class="source">{labelOf(SOURCE_CHOICES, row.source)}</span>
        <span class="device">{deviceLabel(row.deviceName)}</span>
        <span class="message">{row.message}</span>
      </li>
    {:else}
      {#if feed.log !== null}<li class="none">No events</li>{/if}
    {/each}
  </ol>
  {#if skeleton.visible}
    <ol class="rows places" aria-hidden="true" transition:crossFade>
      {#each PLACES as place (place)}
        <li class="row place"><span class="line"></span></li>
      {/each}
    </ol>
  {/if}
</div>

{#if feed.log?.hasNextPage === true}
  <div class="more">
    {#if feed.olderError !== null}<p role="alert">{feed.olderError.message}</p>{/if}
    <button
      type="button"
      tabindex="0"
      disabled={feed.loadingOlder}
      onclick={() => {
        void feed.loadOlder();
      }}
      {@attach press()}>Load older</button
    >
  </div>
{/if}

<style>
  .stack {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
  }

  .rows {
    grid-area: 1 / 1;
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .places {
    background: var(--surface-base);
  }

  /* A placeholder row has a one-line row's height; static, so reduced motion needs no change (5.7.3, Loading). */
  .line {
    inline-size: calc(var(--space-16) * 4);
    max-inline-size: 100%;
    background: var(--surface-raised);
    color: transparent;
  }

  .line::before {
    content: '\200b';
  }

  /* Cells shrink and the message wraps below them on a narrow panel, so a row never widens the page. */
  .row {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--space-1) var(--space-3);
    padding: var(--space-1) var(--space-3);
    border-block-end: var(--border-thin) solid var(--border-strong);
    font-size: var(--text-sm);
  }

  .glow {
    position: absolute;
    inset: 0;
    background: var(--accent);
    opacity: 0;
    pointer-events: none;
  }

  .row > :not(.glow) {
    position: relative;
  }

  .severity {
    font-weight: 600;
  }

  .error {
    color: var(--status-down);
  }

  .warning {
    color: var(--status-warning);
  }

  .info,
  .source,
  .device {
    color: var(--text-secondary);
  }

  time {
    flex: none;
    inline-size: calc(var(--space-16) * 2);
  }

  .severity,
  .source {
    flex: none;
    inline-size: calc(var(--space-16) * 1.5);
  }

  .device {
    flex: 0 1 calc(var(--space-16) * 3);
    min-inline-size: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .message {
    flex: 1 1 calc(var(--space-16) * 3);
    min-inline-size: 0;
    overflow-wrap: anywhere;
    color: var(--text);
  }

  .none {
    padding: var(--space-4);
    color: var(--text-secondary);
  }

  .notice,
  .more {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    color: var(--text);
  }

  .notice p,
  .more p {
    margin: 0;
  }

  .more {
    justify-content: center;
  }

  button {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-overlay);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover:not(:disabled) {
    background: var(--surface-hover);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
