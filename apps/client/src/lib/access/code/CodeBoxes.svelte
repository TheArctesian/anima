<script lang="ts">
  import { onMount } from 'svelte';
  import { arrow, backspace, codeOf, CODE_LENGTH, EMPTY_DIGITS, enter, isComplete, type Entry } from './code-entry';

  interface Props {
    disabled?: boolean;
    /** The sixth box was filled: the page sends the code. */
    oncomplete: (code: string) => void;
  }

  const { disabled = false, oncomplete }: Props = $props();

  let digits = $state(EMPTY_DIGITS);
  let boxes: HTMLInputElement[] = $state([]);

  onMount(() => {
    boxes[0]?.focus();
  });

  function focusBox(index: number): void {
    boxes[index]?.focus();
    boxes[index]?.select();
  }

  function apply(entry: Entry): void {
    digits = entry.digits;
    focusBox(entry.focus);
  }

  function put(index: number, text: string): void {
    const entry = enter(digits, index, text);
    apply(entry);
    if (!isComplete(entry.digits)) return;
    // The code leaves the page state as soon as it is sent (LOG-007).
    oncomplete(codeOf(entry.digits));
    digits = EMPTY_DIGITS;
  }

  function keyed(event: KeyboardEvent, index: number): void {
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    if (event.key === 'Backspace') {
      event.preventDefault();
      apply(backspace(digits, index));
    } else if (event.key === 'ArrowLeft' || event.key === 'ArrowRight') {
      event.preventDefault();
      focusBox(arrow(index, event.key === 'ArrowLeft' ? -1 : 1));
    } else if (event.key.length === 1) {
      // A printable key: a digit is taken here, anything else is dropped.
      event.preventDefault();
      put(index, event.key);
    }
  }

  function pasted(event: ClipboardEvent, index: number): void {
    event.preventDefault();
    put(index, event.clipboardData?.getData('text') ?? '');
  }

  /** What the key events miss: an on-screen keyboard, or the browser's one-time-code autofill, writes the box directly. */
  function written(event: Event & { currentTarget: HTMLInputElement }, index: number): void {
    put(index, event.currentTarget.value);
    event.currentTarget.value = digits[index] ?? '';
  }
</script>

<div class="boxes" role="group" aria-label="Authenticator code">
  {#each digits as digit, index (index)}
    <input
      type="text"
      inputmode="numeric"
      pattern="[0-9]*"
      autocomplete={index === 0 ? 'one-time-code' : 'off'}
      aria-label="Digit {index + 1} of {CODE_LENGTH}"
      value={digit}
      {disabled}
      bind:this={boxes[index]}
      onkeydown={(event) => { keyed(event, index); }}
      onpaste={(event) => { pasted(event, index); }}
      oninput={(event) => { written(event, index); }}
      onfocus={(event) => { event.currentTarget.select(); }}
    />
  {/each}
</div>

<style>
  .boxes {
    display: flex;
    justify-content: center;
    gap: var(--space-2);
  }

  input {
    box-sizing: border-box;
    inline-size: calc(var(--control-lg) + var(--space-1) / 2);
    block-size: var(--control-lg);
    padding: 0;
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-base);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: var(--text-lg);
    font-variant-numeric: tabular-nums;
    text-align: center;
    caret-color: transparent;
  }

  input:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }

  input:disabled {
    opacity: var(--opacity-disabled);
  }
</style>
