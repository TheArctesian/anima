<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';

  interface Props extends Omit<HTMLInputAttributes, 'class' | 'size' | 'children'> {
    /** Height: `control-md` (default) or `control-lg` (sign-in). It matches the buttons and selects beside it (UI-013). */
    size?: 'md' | 'lg';
    /** Machine text: an address, a port, a key, a command. The monospace font, ligatures off (UI-011). */
    mono?: boolean;
    /** The value failed validation: a danger edge and `aria-invalid`. Say why in a Field's error line, not here. */
    invalid?: boolean;
    value?: string | number | null;
    /** An icon inside the field's start, for a search or filter field (UR-7). The field still needs its label. */
    icon?: IconName | undefined;
  }

  let { value = $bindable(), size = 'md', mono = false, invalid = false, type = 'text', icon, ...rest }: Props = $props();
</script>

{#if icon === undefined}
  <input class={[size, mono && 'mono', type === 'number' && 'number']} {type} aria-invalid={invalid || undefined} bind:value {...rest} />
{:else}
  <span class="with-icon">
    <span class="lead"><Icon name={icon} size="sm" /></span>
    <input class={[size, mono && 'mono', type === 'number' && 'number']} {type} aria-invalid={invalid || undefined} bind:value {...rest} />
  </span>
{/if}

<style>
  input {
    box-sizing: border-box;
    block-size: var(--control-md);
    min-inline-size: 0;
    max-inline-size: 100%;
    padding: 0 var(--space-3);
    background: var(--surface-base);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    color: var(--text);
    font: inherit;
    transition: border-color var(--duration-fast) var(--ease-enter);
  }

  .lg {
    block-size: var(--control-lg);
  }

  .with-icon {
    position: relative;
    display: inline-flex;
    max-inline-size: 100%;
  }

  .with-icon input {
    inline-size: 100%;
    padding-inline-start: calc(var(--space-3) + var(--icon-sm) + var(--space-2));
  }

  .lead {
    position: absolute;
    inset-block: 0;
    inset-inline-start: var(--space-3);
    display: inline-flex;
    align-items: center;
    color: var(--text-tertiary);
    pointer-events: none;
  }

  input:focus {
    border-color: var(--accent);
  }

  .mono {
    font-family: var(--font-mono);
    font-variant-ligatures: none;
  }

  .number {
    font-variant-numeric: tabular-nums;
  }

  /* A placeholder is an example, not a value: lighter and slanted, so the two never look alike (finding 23). */
  input::placeholder {
    color: var(--text-tertiary);
    font-style: italic;
    opacity: 1;
  }

  input:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-thin);
  }

  input[aria-invalid='true'] {
    border-color: var(--danger);
  }

  input:disabled {
    opacity: var(--opacity-disabled);
    cursor: not-allowed;
  }
</style>
