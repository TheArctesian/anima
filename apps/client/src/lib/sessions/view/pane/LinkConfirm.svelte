<script lang="ts">
  import Modal from '$lib/forms/modal/Modal.svelte';
  import { press } from '$lib/motion/roles/press';

  interface Props {
    /** The full address to open, or undefined when nothing waits for a confirmation. */
    address: string | undefined;
    onopen: (address: string) => void;
    oncancel: () => void;
  }

  const { address, onopen, oncancel }: Props = $props();
  // The address is kept while the dialog fades out, so it never shows a blank or a changed address.
  let shown = $state('');
  /** The host on its own line, above the whole address: the part an address can disguise (K4 §8.7). */
  const host = $derived(URL.canParse(shown) ? new URL(shown).host : '');
  $effect(() => {
    if (address !== undefined) shown = address;
  });
</script>

<Modal open={address !== undefined} title="Open this link?" onClose={oncancel}>
  <p>The device asked to open this address in your browser. Open it only if you expect it.</p>
  <p class="host" data-host>{host}</p>
  <p class="address" data-address>{shown}</p>
  <div class="actions">
    <button type="button" data-autofocus onclick={oncancel} {@attach press()}>Cancel</button>
    <button type="button" onclick={() => onopen(shown)} {@attach press()}>Open link</button>
  </div>
</Modal>

<style>
  p {
    margin: 0;
  }

  .host {
    font-family: var(--font-mono);
    font-weight: var(--weight-bold);
    overflow-wrap: anywhere;
  }

  .address {
    padding: var(--space-2) var(--space-3);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-base);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    overflow-wrap: anywhere;
    user-select: all;
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    justify-content: flex-end;
  }

  button {
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-4);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }
</style>
