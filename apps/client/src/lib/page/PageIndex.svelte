<script module lang="ts">
  import type { IconName } from '$lib/icons/icons';

  /** One section of a page, as the page's index links to it: the element whose `data-section` is `id`. */
  export interface IndexEntry {
    readonly id: string;
    readonly title: string;
    readonly icon: IconName;
  }
</script>

<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import SmallCaps from './SmallCaps.svelte';

  interface Props {
    entries: readonly IndexEntry[];
    /** The navigation's accessible name, for example "Settings sections". */
    label: string;
    /** A small-caps heading over the links, for example "On this page". */
    heading?: string | undefined;
    /** Below a wide window: a scrolling row of links above the sections, or no index at all. */
    narrow?: 'row' | 'hidden';
  }

  const { entries, label, heading, narrow = 'row' }: Props = $props();
  /** The section nearest the top of the window; the first until the page scrolls. */
  let current = $state<string | null>(null);

  // A section is current once its head passes the top third of the window.
  $effect(() => {
    if (typeof IntersectionObserver !== 'function') return;
    const shown: Record<string, boolean> = {};
    const observer = new IntersectionObserver(
      (changes) => {
        for (const change of changes) {
          const id = (change.target as HTMLElement).dataset['section'] ?? '';
          shown[id] = change.isIntersecting;
        }
        current = entries.find((entry) => shown[entry.id] === true)?.id ?? current;
      },
      { rootMargin: '0px 0px -66% 0px' },
    );
    for (const entry of entries) {
      // The entries' ids are the page's own anchors: plain words and hyphens.
      const section = document.querySelector(`[data-section="${entry.id}"]`);
      if (section !== null) observer.observe(section);
    }
    return () => {
      observer.disconnect();
    };
  });
</script>

<!-- A page's sections (UR-7, SET-001, D17): sticky beside them on a wide window; on a narrow one a scrolling row above
     them, or nothing. -->
<nav aria-label={label} class:hidden={narrow === 'hidden'}>
  {#if heading !== undefined}<span class="heading"><SmallCaps>{heading}</SmallCaps></span>{/if}
  {#each entries as entry (entry.id)}
    <a href="#{entry.id}" aria-current={(current ?? entries[0]?.id) === entry.id ? 'true' : undefined}>
      <Icon name={entry.icon} size="sm" />{entry.title}
    </a>
  {/each}
</nav>

<style>
  nav {
    position: sticky;
    inset-block-start: var(--space-6);
    display: flex;
    flex-direction: column;
    gap: var(--border-focus);
  }

  .heading {
    padding: 0 var(--space-3) var(--space-2);
  }

  a {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    block-size: var(--control-md);
    padding: 0 var(--space-3);
    border-radius: var(--radius-md);
    color: var(--text-tertiary);
    font-size: var(--text-sm);
    text-decoration: none;
    white-space: nowrap;
    transition: color var(--duration-fast) var(--ease-enter), background-color var(--duration-fast) var(--ease-enter);
  }

  a:hover {
    color: var(--text);
  }

  a[aria-current='true'] {
    background: var(--surface-hover);
    color: var(--text);
  }

  a:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: calc(var(--border-focus) * -1);
  }

  @media (max-width: 1100.98px) {
    nav {
      position: static;
      flex-direction: row;
      gap: var(--space-1);
      overflow-x: auto;
      scrollbar-width: none;
    }

    .heading {
      display: none;
    }

    a {
      flex: none;
    }

    .hidden {
      display: none;
    }
  }
</style>
