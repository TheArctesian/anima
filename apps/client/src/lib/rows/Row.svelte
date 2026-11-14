<script lang="ts">
  import type { Snippet } from 'svelte';
  import { resolve } from '$app/paths';
  import type { Pathname } from '$app/types';
  import Icon from '$lib/icons/Icon.svelte';

  interface Props {
    /** The row opens this page of the app: the whole row is the link, with a chevron at its end. */
    to?: Pathname | undefined;
    /** The row's columns, as `grid-template-columns`; one column by default. */
    columns?: string | undefined;
    /** A 2 px status edge at the row's start, for a row that needs a look (D18). */
    edge?: 'down' | 'degraded' | 'warning' | 'maintenance' | 'unknown' | undefined;
    children: Snippet;
  }

  const { to, columns, edge, children }: Props = $props();
</script>

<!-- One hairline row (UR-7, D9). A link row nudges its content toward the chevron on hover. -->
<li class:edge={edge !== undefined} data-edge={edge}>
  {#if to !== undefined}
    <a class="row" href={resolve(to)} style:--row-columns={columns}>
      <span class="cells">{@render children()}</span>
      <span class="go"><Icon name="carbon:chevron-right" size="sm" /></span>
    </a>
  {:else}
    <div class="row" style:--row-columns={columns}><span class="cells">{@render children()}</span></div>
  {/if}
</li>

<style>
  li {
    position: relative;
    border-block-end: var(--border-thin) solid var(--border-subtle);
  }

  .edge::before {
    content: '';
    position: absolute;
    inset-block: var(--space-2);
    inset-inline-start: 0;
    inline-size: var(--border-focus);
    background: var(--status-unknown);
  }

  [data-edge='down']::before {
    background: var(--status-down);
  }

  [data-edge='degraded']::before {
    background: var(--status-degraded);
  }

  [data-edge='warning']::before {
    background: var(--status-warning);
  }

  [data-edge='maintenance']::before {
    background: var(--status-maintenance);
  }

  .row {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    min-block-size: var(--target-min-touch);
    padding: var(--space-2) var(--space-1) var(--space-2) var(--space-2);
    color: inherit;
    text-decoration: none;
  }

  .cells {
    display: grid;
    flex: 1 1 auto;
    grid-template-columns: var(--row-columns, minmax(0, 1fr));
    align-items: center;
    gap: var(--space-4);
    min-inline-size: 0;
  }

  a.row {
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  a.row .cells,
  .go {
    transition: translate var(--duration-base) var(--ease-enter), color var(--duration-fast) var(--ease-enter);
  }

  .go {
    display: inline-flex;
    flex: none;
    color: var(--text-quiet);
  }

  a.row:hover {
    background: linear-gradient(90deg, var(--surface-hover), transparent 70%);
  }

  a.row:hover .cells {
    translate: var(--space-1) 0;
  }

  a.row:hover .go {
    translate: var(--space-1) 0;
    color: var(--accent);
  }

  a.row:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: calc(var(--border-focus) * -1);
  }

  @media (prefers-reduced-motion: reduce) {
    a.row .cells,
    .go {
      transition: color var(--duration-fast) var(--ease-enter);
    }

    a.row:hover .cells,
    a.row:hover .go {
      translate: none;
    }
  }
</style>
