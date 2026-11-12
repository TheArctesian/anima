<script lang="ts">
  import type { HTMLInputAttributes } from 'svelte/elements';

  interface Props extends Omit<HTMLInputAttributes, 'class' | 'size' | 'children'> {
    /** Height: `control-md` (default) or `control-lg` (sign-in). It matches the buttons and selects beside it (UI-013). */
    size?: 'md' | 'lg';
    /** Machine text: an address, a port, a key, a command. Fira Code, ligatures off (UI-011). */
    mono?: boolean;
    /** The value failed validation: a danger edge and `aria-invalid`. Say why in a Field's error line, not here. */
    invalid?: boolean;
    value?: string | number | null;
  }

  let { value = $bindable(), size = 'md', mono = false, invalid = false, type = 'text', ...rest }: Props = $props();
</script>

<input class={[size, mono && 'mono', type === 'number' && 'number']} {type} aria-invalid={invalid || undefined} bind:value {...rest} />

<style>
  input {
    box-sizing: border-box;
    block-size: var(--control-md);
    min-inline-size: 0;
    padding: 0 var(--space-3);
    background: var(--surface-sunken);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    color: var(--text);
    font: inherit;
    transition: border-color var(--duration-fast) var(--ease-enter);
  }

  .lg {
    block-size: var(--control-lg);
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
