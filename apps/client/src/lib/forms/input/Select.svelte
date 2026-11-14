<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLSelectAttributes } from 'svelte/elements';
  import Icon from '$lib/icons/Icon.svelte';

  interface Props extends Omit<HTMLSelectAttributes, 'class' | 'size' | 'children' | 'multiple'> {
    /** The `<option>` elements. */
    children: Snippet;
    size?: 'md' | 'lg';
    invalid?: boolean;
    mono?: boolean;
  }

  let { value = $bindable(), children, size = 'md', invalid = false, mono = false, ...rest }: Props = $props();
</script>

<span class="select {size}">
  <select class={[mono && 'mono']} aria-invalid={invalid || undefined} bind:value {...rest}>{@render children()}</select>
  <span class="chevron"><Icon name="carbon:chevron-down" size="sm" /></span>
</span>

<style>
  .select {
    position: relative;
    display: inline-block;
    max-inline-size: 100%;
    block-size: var(--control-md);
  }

  .lg {
    block-size: var(--control-lg);
  }

  select {
    box-sizing: border-box;
    inline-size: 100%;
    block-size: 100%;
    padding: 0 calc(var(--space-3) + var(--icon-sm) + var(--space-2)) 0 var(--space-3);
    appearance: none;
    background: var(--surface-sunken);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    color: var(--text);
    font: inherit;
  }

  .mono {
    font-family: var(--font-mono);
    font-variant-ligatures: none;
  }

  select:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-thin);
  }

  select[aria-invalid='true'] {
    border-color: var(--danger);
  }

  select:disabled {
    opacity: var(--opacity-disabled);
    cursor: not-allowed;
  }

  .chevron {
    position: absolute;
    inset-block: 0;
    inset-inline-end: var(--space-3);
    display: flex;
    align-items: center;
    color: var(--text-secondary);
    pointer-events: none;
  }
</style>
