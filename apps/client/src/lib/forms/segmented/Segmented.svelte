<script lang="ts" generics="Value extends string">
  import { tick } from 'svelte';
  import { movedTo } from './segmented';

  interface Option {
    value: Value;
    label: string;
    /** How many things this option would show, in tabular figures (UI-025). */
    count?: number;
  }

  interface Props {
    /** The group's accessible name: what the choice is about, for example "Status". */
    label: string;
    options: readonly Option[];
    value: Value;
    size?: 'sm' | 'md' | 'lg';
    onchange?: (value: Value) => void;
  }

  /*
   * One choice from a few, so it is a radio group, not a row of toggle buttons: `aria-pressed` says "on or off",
   * and a screen reader would read the row as independent switches. Radios also give arrow keys, which move and
   * choose together, and one tab stop (the chosen option).
   */
  let { label, options, value = $bindable(), size = 'md', onchange }: Props = $props();
  let radios: (HTMLButtonElement | undefined)[] = $state([]);

  function choose(next: Value): void {
    if (next === value) return;
    value = next;
    onchange?.(next);
  }

  async function onkeydown(event: KeyboardEvent, index: number): Promise<void> {
    const target = movedTo(event.key, index, options.length);
    const option = target === undefined ? undefined : options[target];
    if (target === undefined || option === undefined) return;
    event.preventDefault();
    choose(option.value);
    await tick();
    radios[target]?.focus();
  }
</script>

<div class="segmented {size}" role="radiogroup" aria-label={label}>
  {#each options as option, index (option.value)}
    <button
      type="button"
      role="radio"
      aria-checked={option.value === value}
      tabindex={option.value === value ? 0 : -1}
      bind:this={radios[index]}
      onclick={() => {
        choose(option.value);
      }}
      onkeydown={(event) => onkeydown(event, index)}
    >
      {option.label}{#if option.count !== undefined}<span class="count">{option.count}</span>{/if}
    </button>
  {/each}
</div>

<style>
  .segmented {
    box-sizing: border-box;
    display: inline-flex;
    gap: var(--space-1);
    block-size: var(--control-md);
    padding: var(--space-1);
    background: var(--surface-sunken);
    box-shadow: inset 0 0 0 var(--border-thin) var(--border-subtle);
    border-radius: var(--radius-md);
  }

  .sm {
    block-size: var(--control-sm);
  }

  .lg {
    block-size: var(--control-lg);
  }

  button {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0 var(--space-3);
    background: none;
    border: 0;
    border-radius: var(--radius-sm);
    color: var(--text-secondary);
    font: inherit;
    font-size: var(--text-sm);
    white-space: nowrap;
    transition:
      background-color var(--duration-fast) var(--ease-enter),
      color var(--duration-fast) var(--ease-enter);
  }

  button:hover:not([aria-checked='true']) {
    color: var(--text);
  }

  button[aria-checked='true'] {
    background: var(--surface-raised);
    box-shadow: 0 0 0 var(--border-thin) var(--border-subtle);
    color: var(--text);
    font-weight: var(--weight-bold);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-thin);
  }

  .count {
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
    font-weight: var(--weight-regular);
  }
</style>
