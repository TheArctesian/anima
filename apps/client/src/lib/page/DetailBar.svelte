<script lang="ts">
  import type { Snippet } from 'svelte';
  import { resolve } from '$app/paths';
  import type { Pathname } from '$app/types';
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import { sharedItem } from '$lib/motion/shared/shared';
  import { pageTitle } from './title';

  interface Props {
    /** The list this page belongs to: "Devices" at "/devices". */
    back: { readonly to: Pathname; readonly label: string };
    /** The thing's kind or system, in the accent colour: a device type, a session's protocol. */
    icon: IconName;
    /**
     * The thing's name: the visible `h1`, and the window title "<title> – Anima" (NAV-010). `null` while it loads, or
     * when there is no such thing: only the back link shows, and a placeholder keeps the row's height (D-13, MOT-007).
     */
    title: string | null;
    /** The key of the shared-element morph the name flies from, such as the list row's name (MOT-005). */
    shared?: string | undefined;
    /** Status and identity beside the name: a status mark, a host in mono. Stays while the bar is pinned. */
    meta?: Snippet;
    /** The page's actions at the end of the row: Connect, a split button, an overflow menu. */
    actions?: Snippet;
    /** Why the status is what it is, and the fix. Folds away while the bar is pinned. */
    reason?: Snippet;
    /** The page's tabs, the bar's last row. */
    tabs?: Snippet;
  }

  const { back, icon, title, shared, meta, actions, reason, tabs }: Props = $props();
  let sentinel = $state<HTMLElement>();
  let stuck = $state(false);

  // The bar is pinned once the line just above it has scrolled out of the window.
  $effect(() => {
    if (sentinel === undefined || typeof IntersectionObserver !== 'function') return;
    const observer = new IntersectionObserver(([entry]) => {
      stuck = entry !== undefined && !entry.isIntersecting && entry.boundingClientRect.top < 0;
    });
    observer.observe(sentinel);
    return () => {
      observer.disconnect();
    };
  });
</script>

<svelte:head><title>{title === null ? 'Anima' : pageTitle(title)}</title></svelte:head>

<div class="sentinel" aria-hidden="true" bind:this={sentinel}></div>
<!-- The detail bar (UR-7, D16): every detail page's header. The list it came from, the thing's icon and name, its
     status, its actions and its tabs, pinned to the top of the window as the page scrolls. -->
<header class:stuck>
  <div class="top">
    <nav class="crumb" aria-label="Breadcrumb">
      <a href={resolve(back.to)}>{back.label}</a>
      {#if title === null}
        <span class="placeholder" aria-hidden="true"></span>
      {:else}
        <span class="slash" aria-hidden="true">/</span>
        <span class="mark"><Icon name={icon} size="md" /></span>
        <h1 {title} {@attach shared === undefined ? undefined : sharedItem(shared)}>{title}</h1>
      {/if}
    </nav>
    {#if meta}<div class="meta">{@render meta()}</div>{/if}
    {#if actions}<div class="actions">{@render actions()}</div>{/if}
  </div>
  {#if reason}
    <div class="fold" inert={stuck}><div><div class="reason">{@render reason()}</div></div></div>
  {/if}
  {#if tabs}<div class="tabs">{@render tabs()}</div>{/if}
</header>

<style>
  .sentinel {
    block-size: 0;
  }

  /* The bar runs to the page's edges, over the main column's padding, so the page scrolls cleanly under it. */
  header {
    position: sticky;
    inset-block-start: 0;
    z-index: var(--layer-sticky);
    display: flex;
    flex-direction: column;
    margin: calc(var(--space-3) * -1) calc(var(--page-gutter, var(--space-6)) * -1) var(--space-4);
    padding: var(--space-3) var(--page-gutter, var(--space-6)) 0;
    border-block-end: var(--border-thin) solid var(--border-subtle);
    background: var(--surface-base);
  }

  /* Below the compact breakpoint the unreachable strip takes the top of the window; the bar pins under it. */
  :global(:root[data-nav='icons'][data-hub-banner]) header {
    inset-block-start: var(--nav-banner-block-size);
  }

  header:not(:has(.tabs)) {
    padding-block-end: var(--space-3);
  }

  /* A container, so an action can fit its words to the bar's width, as "Set up screen sharing" does (R-079). */
  .top {
    container-type: inline-size;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-4);
    min-block-size: var(--control-md);
  }

  .crumb {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-inline-size: 0;
  }

  .crumb a {
    color: var(--text-tertiary);
    text-decoration: none;
    transition: color var(--duration-fast) var(--ease-enter);
  }

  .crumb a:hover {
    color: var(--text);
  }

  .crumb a:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
    border-radius: var(--radius-sm);
  }

  .slash {
    color: var(--text-quiet);
  }

  .mark {
    display: inline-flex;
    color: var(--accent);
  }

  /* The name's place while it loads: as tall as the name, and empty, so nothing flashes (D-13, MOT-007). */
  .placeholder {
    block-size: calc(var(--text-xl) * var(--leading-tight));
  }

  h1 {
    margin: 0;
    overflow: hidden;
    font-size: var(--text-xl);
    font-weight: var(--weight-medium);
    line-height: var(--leading-tight);
    letter-spacing: var(--tracking-display);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-4);
    min-inline-size: 0;
    color: var(--text-tertiary);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    margin-inline-start: auto;
  }

  /* The reason folds from its height to none as the bar pins (UR-7, D16), so the pinned bar is one row and the tabs. */
  .fold {
    display: grid;
    grid-template-rows: 1fr;
    transition:
      grid-template-rows var(--duration-base) var(--ease-enter),
      opacity var(--duration-fast) var(--ease-enter);
  }

  .fold > div {
    min-block-size: 0;
    overflow: hidden;
  }

  .stuck .fold {
    grid-template-rows: 0fr;
    opacity: 0;
  }

  .reason {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
    padding-block-start: var(--space-2);
    color: var(--text-tertiary);
    font-size: var(--text-sm);
  }

  .tabs {
    margin-block-start: var(--space-3);
  }

  /* The strip sits on the bar's own rule: its underline and gap give way, and the selected tab's line overlaps the rule. */
  .tabs > :global(*) {
    margin-block-end: calc(var(--border-thin) * -1);
    box-shadow: none;
  }

  @media (prefers-reduced-motion: reduce) {
    .fold {
      transition: none;
    }
  }

  /* On a phone the actions take their own row under the name, and leave while the bar is pinned. */
  @media (max-width: 767.98px) {
    h1 {
      font-size: var(--text-lg);
    }

    .actions {
      margin-inline-start: 0;
      inline-size: 100%;
    }

    .stuck .actions {
      display: none;
    }
  }
</style>
