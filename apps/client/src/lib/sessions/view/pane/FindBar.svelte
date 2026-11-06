<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';

  interface Props {
    /** Select the next (or previous) match of `text` in the terminal; false when there is none. */
    onfind: (text: string, backwards: boolean) => boolean;
    /** Close the bar; the owner clears the match and returns the focus to the terminal. */
    onclose: () => void;
  }

  const { onfind, onclose }: Props = $props();
  let text = $state('');
  let missing = $state(false);

  function find(backwards: boolean) {
    missing = text !== '' && !onfind(text, backwards);
  }

  function key(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      onclose();
    } else if (event.key === 'Enter') {
      event.preventDefault();
      find(event.shiftKey);
    }
  }

  function focusInput(input: HTMLInputElement) {
    input.focus();
    input.select();
  }
</script>

<div class="find" role="search" aria-label="Find in terminal">
  <input
    type="text"
    aria-label="Find in terminal"
    placeholder="Find"
    autocomplete="off"
    spellcheck="false"
    enterkeyhint="search"
    bind:value={text}
    oninput={() => (missing = false)}
    onkeydown={key}
    {@attach focusInput}
  />
  <button type="button" title="Previous match (Shift+Enter)" onclick={() => find(true)} {@attach press()}>
    Previous
  </button>
  <button type="button" title="Next match (Enter)" onclick={() => find(false)} {@attach press()}>
    Next
  </button>
  <button type="button" aria-label="Close find" title="Close (Escape)" onclick={onclose} {@attach press()}>
    <Icon name="nf-md-close" size="sm" />
  </button>
  <span class="status" role="status">{missing ? 'No match' : ''}</span>
</div>

<style>
  .find {
    display: flex;
    flex: 1;
    align-items: center;
    gap: var(--space-1);
    min-inline-size: 0;
  }

  input {
    flex: 1;
    box-sizing: border-box;
    min-inline-size: 0;
    max-inline-size: calc(var(--space-16) * 4);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-2);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--bg);
    color: var(--text);
    font: inherit;
    font-family: var(--font-mono);
  }

  button {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: center;
    min-inline-size: var(--target-min-desktop);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-2);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
  }

  input:focus-visible,
  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }

  .status {
    flex: none;
    color: var(--text-muted);
    font-size: var(--text-sm);
    white-space: nowrap;
  }
</style>
