<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';

  interface Props {
    /** The device's address, as the hub gives it. */
    host: string;
  }

  const { host }: Props = $props();
  let copy = $state<'idle' | 'copied' | 'failed'>('idle');

  async function copyHost(): Promise<void> {
    try {
      await navigator.clipboard.writeText(host);
      copy = 'copied';
    } catch {
      // No clipboard, or the system refused it; the address stays selectable.
      copy = 'failed';
    }
  }
</script>

<!-- A device's address with one button that copies it, as on the phone's device page (MOB-015). The button's name says
     the outcome, so a screen reader hears it without a second live region. -->
<span class="address">
  <span class="mono">{host}</span>
  <button
    type="button"
    tabindex="0"
    class="copy"
    aria-label={copy === 'copied' ? 'Address copied' : copy === 'failed' ? 'Could not copy the address' : 'Copy address'}
    title={copy === 'copied' ? 'Address copied' : 'Copy address'}
    onclick={() => void copyHost()}
    {@attach press()}><Icon name={copy === 'copied' ? 'carbon:checkmark' : 'carbon:copy'} size="sm" /></button
  >
</span>

<style>
  .address {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-inline-size: 0;
  }

  .mono {
    overflow: hidden;
    font-family: var(--font-mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .copy {
    display: inline-grid;
    flex: none;
    place-items: center;
    inline-size: var(--control-sm);
    block-size: var(--control-sm);
    padding: 0;
    border: 0;
    border-radius: var(--radius-md);
    background: none;
    color: var(--text-secondary);
    cursor: pointer;
  }

  .copy:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  .copy:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: calc(var(--border-focus) * -1);
  }
</style>
