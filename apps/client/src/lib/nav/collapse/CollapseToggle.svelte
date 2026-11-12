<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';

  interface Props {
    /** The bar shows icons only because the operator collapsed it. */
    collapsed: boolean;
    ontoggle: () => void;
  }

  const { collapsed, ontoggle }: Props = $props();
  const label = $derived(collapsed ? 'Expand sidebar' : 'Collapse sidebar');
</script>

<button
  type="button"
  aria-label={label}
  aria-expanded={!collapsed}
  aria-controls="main-nav"
  title={label}
  onclick={ontoggle}
  {@attach press()}
>
  <Icon name="nf-md-dock_left" />
</button>

<style>
  button {
    display: inline-flex;
    flex: none;
    align-items: center;
    justify-content: center;
    inline-size: var(--target-min-touch);
    block-size: var(--target-min-touch);
    padding: 0;
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text-secondary);
    cursor: pointer;
    transition:
      background-color var(--duration-fast) var(--ease-enter),
      color var(--duration-fast) var(--ease-enter);
  }

  button:hover {
    background: var(--surface-overlay);
    color: var(--text);
  }

  button:active {
    transition-duration: var(--duration-press);
    background: var(--surface-hover);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
