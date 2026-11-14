<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import type { IndexEntry } from './index';

  interface Props {
    entries: readonly IndexEntry[];
  }

  const { entries }: Props = $props();
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
    for (const section of document.querySelectorAll('[data-section]')) observer.observe(section);
    return () => {
      observer.disconnect();
    };
  });
</script>

<!-- The page's sections (UR-7, SET-001): sticky beside them on a wide window, a scrolling row above them on a narrow one. -->
<nav aria-label="Settings sections">
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

    a {
      flex: none;
    }
  }
</style>
