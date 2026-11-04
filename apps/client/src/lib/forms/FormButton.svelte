<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import { press } from '$lib/motion/roles/press';

  interface Props extends Omit<HTMLButtonAttributes, 'class' | 'children'> {
    /** `danger` is for the button that destroys something. */
    tone?: 'plain' | 'primary' | 'danger';
    children: Snippet;
  }

  const { tone = 'plain', type = 'button', children, ...rest }: Props = $props();
</script>

<button class={tone} {type} {...rest} {@attach press()}>{@render children()}</button>

<style>
  button {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-4);
    background: var(--surface-raised);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-press) var(--ease-enter);
  }

  button:hover:not(:disabled, [aria-disabled='true']) {
    background: var(--border);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  button:disabled,
  button[aria-disabled='true'] {
    color: var(--text-muted);
  }

  .primary {
    background: var(--accent-deep);
    border-color: var(--accent-deep);
  }

  .danger {
    background: var(--status-down-tint);
    border-color: var(--status-down);
    color: var(--status-down);
  }
</style>
