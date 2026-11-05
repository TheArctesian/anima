<script lang="ts">
  import { press } from '$lib/motion/roles/press';
  import type { LogFeed } from './feed.svelte';
  import { newRowMotion } from './rows.svelte';
  import { SEVERITY_CHOICES, SOURCE_CHOICES, deviceLabel, labelOf, timeLabel } from './view';

  interface Props {
    feed: LogFeed;
    now: () => Date;
  }

  const { feed, now }: Props = $props();
  const rows = $derived(feed.log?.rows ?? []);
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
{#if feed.liveError !== null}
  <p class="notice" role="status">Live logs stopped: {feed.liveError.message}</p>
{/if}

<ol class="rows" aria-label="Log events" aria-busy={feed.log === null && feed.error === null}>
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
  .rows {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  /* Cells shrink and the message wraps below them on a narrow panel, so a row never widens the page. */
  .row {
    position: relative;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: var(--space-1) var(--space-3);
    padding: var(--space-1) var(--space-3);
    border-block-end: var(--border-thin) solid var(--border);
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
    color: var(--text-muted);
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
    color: var(--text-muted);
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
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover:not(:disabled) {
    background: var(--border);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
