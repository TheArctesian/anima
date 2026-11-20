<script module lang="ts">
  import type { IconName } from '$lib/icons/icons';

  /** One section of a page, as the page's index links to it: the element whose `data-section` is `id`. */
  export interface IndexEntry {
    readonly id: string;
    readonly title: string;
    readonly icon: IconName;
    /** The group of the section, for example "This computer": the index writes it once, above the first link of the group. */
    readonly group?: string;
  }
</script>

<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import QuietLabel from '../QuietLabel.svelte';

  interface Props {
    entries: readonly IndexEntry[];
    /** The navigation's accessible name, for example "Settings sections". */
    label: string;
    /** A quiet heading over the links, for example "On this page". */
    heading?: string | undefined;
    /** Below a wide window: a scrolling row, a vertical list, or a hidden index. */
    narrow?: 'row' | 'hidden' | 'stacked';
  }

  const { entries, label, heading, narrow = 'row' }: Props = $props();
  /** The section nearest the top of the window; the first until the page scrolls. */
  let current = $state<string | null>(null);

  // The current section is the last one whose head has passed the top tenth of the window, so a link that brings a short
  // section to the top marks that section; at the foot of the page it is the last section, which may never get that high.
  // It is worked out again on every scroll of the page, whichever element scrolls (a few reads of layout).
  $effect(() => {
    function update(): void {
      const line = innerHeight / 10;
      const page = document.scrollingElement ?? document.documentElement;
      const atFoot = page.scrollTop > 0 && page.scrollTop + innerHeight >= page.scrollHeight - 1;
      let next = entries[0]?.id ?? null;
      for (const entry of entries) {
        // The entries' ids are the page's own anchors: plain words and hyphens.
        const section = document.querySelector(`[data-section="${entry.id}"]`);
        if (section !== null && (atFoot || section.getBoundingClientRect().top <= line)) next = entry.id;
      }
      current = next;
    }
    update();
    document.addEventListener('scroll', update, { capture: true, passive: true });
    return () => {
      document.removeEventListener('scroll', update, { capture: true });
    };
  });
</script>

<!-- A page's sections (UR-7, SET-001, D17): sticky beside them on a wide window; on a narrow one a scrolling row, a
     vertical list above them, or nothing. -->
<nav aria-label={label} class:hidden={narrow === 'hidden'} class:stacked={narrow === 'stacked'}>
  {#if heading !== undefined}<span class="heading"><QuietLabel>{heading}</QuietLabel></span>{/if}
  {#each entries as entry, at (entry.id)}
    {#if entry.group !== undefined && entry.group !== entries[at - 1]?.group}<span class="group"><QuietLabel>{entry.group}</QuietLabel></span>{/if}
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

  /* The name of a group of links; each group after the first has space above it (R-224). */
  .group {
    padding: var(--space-2) var(--space-3) var(--space-1);
  }

  a + .group {
    margin-block-start: var(--space-3);
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

    nav.stacked {
      flex-direction: column;
      overflow: visible;
    }

    .stacked a {
      min-block-size: var(--target-min-touch);
      block-size: auto;
      padding-block: var(--space-2);
      white-space: normal;
    }

    .heading,
    .group {
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
