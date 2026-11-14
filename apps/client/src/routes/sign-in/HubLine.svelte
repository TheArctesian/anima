<script lang="ts">
  import { tick } from 'svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import TextField from './TextField.svelte';

  interface Props {
    host: string;
    /** The web client: the hub is the one that served the page, so it is told but cannot be changed (CLI-002). */
    fixed?: boolean;
    editing: boolean;
    text: string;
    problem: string | undefined;
    /** Enter in the field: save the address now. */
    onsave: () => void;
    /** Escape: the page restores the current address. */
    oncancel: () => void;
  }

  let { host, fixed = false, editing = $bindable(), text = $bindable(), problem, onsave, oncancel }: Props = $props();
  let field: HTMLInputElement | undefined = $state();
  let change: HTMLButtonElement | undefined = $state();

  // The field exists only while editing, so focus moves into it the moment it appears.
  $effect(() => {
    field?.focus();
  });

  async function keyed(event: KeyboardEvent): Promise<void> {
    if (event.key !== 'Escape') return;
    event.preventDefault();
    oncancel();
    await tick();
    change?.focus();
  }
</script>

<!-- Only the desktop app can change its hub; a browser talks only to the hub that served the page (CLI-002, NAV-009). -->
{#if fixed}
  <p class="line"><Icon name="carbon:network-3" size="sm" />Hub: <code>{host}</code></p>
{:else}
  <form
    class="hub"
    novalidate
    onsubmit={(event) => {
      event.preventDefault();
      onsave();
    }}
  >
    {#if editing}
      <TextField id="hub-address" name="hubAddress" label="Hub address" autocomplete="off" bind:value={text} bind:input={field} error={problem} onkeydown={keyed} />
    {:else}
      <p class="line">
        <Icon name="carbon:network-3" size="sm" />Hub: <code>{host}</code>
        <!-- tabindex="0": WebKit skips plain buttons when Tab is pressed unless they are focusable (NAV-005). -->
        <button type="button" tabindex="0" bind:this={change} onclick={() => { editing = true; }} {@attach press()}>Change</button>
      </p>
    {/if}
  </form>
{/if}

<style>
  .hub {
    inline-size: 100%;
  }

  .line {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    margin: 0;
    color: var(--text-tertiary);
    font-size: var(--text-sm);
  }

  code {
    font-family: var(--font-mono);
    color: var(--text-secondary);
  }

  button {
    min-block-size: var(--control-sm);
    padding: 0 var(--space-2);
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--accent);
    font: inherit;
    cursor: pointer;
  }

  button:hover {
    background: var(--surface-raised);
    color: var(--accent-hover);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }
</style>
