<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import { itemIcon, type HeaderText } from './header';

  interface Props {
    items: readonly HeaderText[];
    /** The kind's icon, shown on the kind chip. */
    kind: IconName;
    /** The operating system's logo, shown again on the OS chip. */
    os: IconName;
  }

  const { items, kind, os }: Props = $props();
</script>

<!-- Host, IP, kind, OS and tags (DEV-020): each a chip with an icon and its words, so none is said by the icon alone (UI-003). -->
{#each items as item, index (index)}
  <li class="chip {item.part}" title={item.text}>
    <Icon name={itemIcon(item, kind, os)} size="sm" /><span class="text">{item.text}</span>
  </li>
{/each}

<style>
  .chip {
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    max-inline-size: 100%;
    min-block-size: var(--chip-block-size);
    padding: 0 var(--space-2);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-pill);
    background: var(--surface);
    color: var(--text-muted);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  /* A chip wraps onto the next line whole; only a single word wider than the header breaks (DEV-021). */
  .text {
    min-inline-size: 0;
    overflow-wrap: anywhere;
  }

  .host .text,
  .ip .text {
    font-family: var(--font-mono);
  }
</style>
