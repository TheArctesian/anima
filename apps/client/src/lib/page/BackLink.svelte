<script lang="ts">
  import { resolve } from '$app/paths';
  import type { Pathname } from '$app/types';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';

  interface Props {
    /** The place it goes to, a path of the app such as "/devices". */
    to: Pathname;
    /** The place it returns to, for example "Devices". */
    label: string;
  }

  const { to, label }: Props = $props();
</script>

<!-- A ghost link: a chevron and the word, with a target of at least 24 px (UI-013, finding O8). -->
<a href={resolve(to)} {@attach press()}><Icon name="carbon:chevron-left" size="sm" />{label}</a>

<style>
  a {
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-block-size: var(--target-min-desktop);
    min-inline-size: var(--target-min-desktop);
    padding-inline: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    text-decoration: none;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  a:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  a:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }
</style>
