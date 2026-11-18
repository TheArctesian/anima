<script lang="ts">
  import { tick } from 'svelte';
  import { NAME_DEBOUNCE_MS, debounced } from '$lib/devices/toolbar/toolbar';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import '$lib/nav/phone/head.css';

  interface Props {
    /** The search text of the log filter. */
    text: string;
    onTextChange: (text: string) => void;
  }

  const { text, onTextChange }: Props = $props();
  let open = $state(false);
  let typed = $state('');
  let field: HTMLInputElement | undefined = $state();
  const send = debounced(() => {
    onTextChange(typed);
  }, NAME_DEBOUNCE_MS);

  $effect(() => {
    typed = text;
    if (text !== '') open = true;
  });
  $effect(() => send.cancel);

  /** The button opens the field and puts the focus in it; a second press closes it and clears the search. */
  async function toggle(): Promise<void> {
    open = !open;
    if (open) {
      await tick();
      field?.focus();
      return;
    }
    send.cancel();
    if (text !== '') onTextChange('');
  }
</script>

<!-- The head of the Logs page in the phone layout (mockup 6): the title and a button that opens a search (LOG-011). -->
<div class="pagehead">
  <h1>Logs</h1>
  <!-- tabindex="0": WebKit skips plain buttons when Tab is pressed unless they are focusable (NAV-005). -->
  <button type="button" class="phone-head-button" tabindex="0" aria-label="Search logs" aria-expanded={open} onclick={() => void toggle()} {@attach press()}><Icon name={open ? 'carbon:close' : 'carbon:search'} size="lg" /></button>
</div>
{#if open}
  <label class="phone-search" role="search">
    <span class="label">Search logs</span>
    <Icon name="carbon:search" size="lg" />
    <input
      bind:this={field}
      type="search"
      placeholder="Search"
      maxlength="200"
      value={typed}
      oninput={(event) => {
        typed = event.currentTarget.value;
        send();
      }}
    />
  </label>
{/if}
