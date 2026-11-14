<script lang="ts">
  import { resolve } from '$app/paths';
  import Icon from '$lib/icons/Icon.svelte';
  import LiveNotice from '$lib/hub/notice/LiveNotice.svelte';
  import { skeletonDelay } from '$lib/motion/loading/delay.svelte';
  import type { LoadingClock } from '$lib/motion/loading/loading';
  import { press } from '$lib/motion/roles/press';
  import { crossFade } from '$lib/motion/roles/transitions';
  import type { LogFeed } from './feed.svelte';
  import { newRowMotion } from './rows.svelte';
  import { SEVERITY_CHOICES, SOURCE_CHOICES, clockLabel, dayLabel, deviceLabel, labelOf, sourceIcon } from './view';

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
  /** Each row with the day heading it opens, if it is the first of its day. */
  const days = $derived(rows.map((row, index) => {
    const day = dayLabel(row.at, now());
    const before = rows[index - 1];
    return before !== undefined && dayLabel(before.at, now()) === day ? null : day;
  }));
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
    {#each rows as row, index (row.id)}
      {@const day = days[index]}
      {#if day !== null && day !== undefined && day !== ''}<li class="day" aria-hidden="true">{day}</li>{/if}
      <li class="row {row.severity.toLowerCase()}" {@attach slide(row.id)}>
        <span class="glow" data-glow aria-hidden="true"></span>
        <time datetime={row.at}>{clockLabel(row.at)}</time>
        <span class="severity"><span class="dot" aria-hidden="true"></span>{labelOf(SEVERITY_CHOICES, row.severity)}</span>
        <span class="source"><Icon name={sourceIcon(row.source)} size="sm" />{labelOf(SOURCE_CHOICES, row.source)}</span>
        <span class="device"
          >{#if typeof row.deviceId === 'string' && typeof row.deviceName === 'string'}<a
              href={resolve('/devices/[id]', { id: row.deviceId })}>{row.deviceName}</a
            >{:else}{deviceLabel(row.deviceName)}{/if}</span
        >
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
    background: var(--surface-raised);
  }

  /* A placeholder row has a one-line row's height; static, so reduced motion needs no change (5.7.3, Loading). */
  .line {
    inline-size: calc(var(--space-16) * 4);
    max-inline-size: 100%;
    background: var(--surface-overlay);
    color: transparent;
  }

  .line::before {
    content: '\200b';
  }

  /* Each day's rows sit under its name, which stays at the top while its rows scroll (UR-7). */
  .day {
    position: sticky;
    inset-block-start: 0;
    z-index: var(--layer-sticky);
    padding: var(--space-3) var(--space-2) var(--space-2);
    border-block-end: var(--border-thin) solid var(--border-subtle);
    background: var(--surface-raised);
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
  }

  /* Hairline rows in fixed columns; on a narrow panel the cells wrap and the message goes below, so a row never widens the page. */
  .row {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--space-1) var(--space-4);
    padding: var(--space-2);
    border-block-end: var(--border-thin) solid var(--border-subtle);
    font-size: var(--text-sm);
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  .row:hover {
    background: var(--surface-hover);
  }

  /* What needs you carries an edge in its colour, beside the word, never the colour alone. */
  .row.error {
    box-shadow: inset var(--border-focus) 0 0 var(--status-down);
  }

  .row.warning {
    box-shadow: inset var(--border-focus) 0 0 var(--status-warning);
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

  time {
    flex: none;
    inline-size: calc(var(--space-16) * 1.25);
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
  }

  .severity,
  .source {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: var(--space-2);
    inline-size: calc(var(--space-16) * 1.5);
    color: var(--text-tertiary);
  }

  .dot {
    inline-size: var(--space-2);
    block-size: var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--text-quiet);
  }

  .error .severity {
    color: var(--status-down-text);
  }

  .warning .severity {
    color: var(--status-warning-text);
  }

  .error .dot {
    background: var(--status-down);
  }

  .warning .dot {
    background: var(--status-warning);
  }

  .source :global(svg) {
    color: var(--text-quiet);
  }

  .device {
    flex: 0 1 calc(var(--space-16) * 2);
    min-inline-size: 0;
    overflow: hidden;
    color: var(--text-tertiary);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .device a {
    color: var(--text-secondary);
    text-decoration: none;
  }

  .device a:hover {
    color: var(--text);
    text-decoration: underline;
    text-underline-offset: var(--space-1);
  }

  .device a:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-thin);
  }

  .message {
    flex: 1 1 calc(var(--space-16) * 3);
    min-inline-size: 0;
    overflow-wrap: anywhere;
    color: var(--text);
  }

  .none {
    padding: var(--space-6) var(--space-2);
    color: var(--text-tertiary);
    font-size: var(--text-sm);
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
