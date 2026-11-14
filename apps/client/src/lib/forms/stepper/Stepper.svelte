<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { stepped } from './stepper';

  interface Props {
    /** The number as text, so a half-typed value is never read as a number; empty means none. */
    value: string;
    /** The accessible name of the number field. */
    label: string;
    min: number;
    max: number;
    /** Shown while the value is empty; one step down from `min` empties it. Without it the value stays at `min` or above. */
    placeholder?: string | undefined;
    id?: string | undefined;
    invalid?: boolean;
    disabled?: boolean;
    'aria-describedby'?: string | undefined;
    /** Runs after a step at once, and after typing when the field is left or Return is pressed (UR-7, D17). */
    oncommit: () => void;
  }

  let {
    value = $bindable(),
    label,
    min,
    max,
    placeholder,
    id,
    invalid = false,
    disabled = false,
    'aria-describedby': describedBy,
    oncommit,
  }: Props = $props();

  function step(by: 1 | -1): void {
    if (disabled) return;
    const next = stepped(value, by, { min, max, empty: placeholder !== undefined });
    if (next === value) return;
    value = next;
    oncommit();
  }
</script>

<span class="stepper" class:invalid>
  <button type="button" tabindex="0" aria-label="One fewer" aria-disabled={disabled || undefined} onclick={() => { step(-1); }}>
    <Icon name="carbon:subtract" size="sm" />
  </button>
  <input
    {id}
    bind:value
    inputmode="numeric"
    autocomplete="off"
    aria-label={label}
    aria-invalid={invalid || undefined}
    aria-describedby={describedBy}
    {placeholder}
    {disabled}
    onblur={oncommit}
    onkeydown={(event) => {
      if (event.key === 'Enter') {
        event.preventDefault();
        oncommit();
      }
    }}
  />
  <button type="button" tabindex="0" aria-label="One more" aria-disabled={disabled || undefined} onclick={() => { step(1); }}>
    <Icon name="carbon:add" size="sm" />
  </button>
</span>

<style>
  .stepper {
    box-sizing: border-box;
    display: inline-flex;
    block-size: var(--control-md);
    overflow: hidden;
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-base);
  }

  .stepper:focus-within {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-thin);
  }

  .invalid {
    border-color: var(--danger);
  }

  button {
    display: inline-grid;
    place-items: center;
    inline-size: var(--control-md);
    padding: 0;
    border: 0;
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
  }

  button:hover:not([aria-disabled='true']) {
    background: var(--surface-hover);
    color: var(--text);
  }

  button[aria-disabled='true'] {
    opacity: var(--opacity-disabled);
    cursor: not-allowed;
  }

  button:focus-visible {
    outline: none;
    background: var(--surface-hover);
    color: var(--text);
  }

  input {
    box-sizing: border-box;
    inline-size: calc(var(--space-16) + var(--space-2));
    padding: 0 var(--space-1);
    border: 0;
    border-inline: var(--border-thin) solid var(--border-subtle);
    background: none;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
    text-align: center;
  }

  input:focus {
    outline: none;
  }

  input::placeholder {
    color: var(--text-tertiary);
    font-family: var(--font-ui);
    font-style: italic;
    opacity: 1;
  }
</style>
