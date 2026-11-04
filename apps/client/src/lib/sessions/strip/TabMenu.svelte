<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { popover } from '$lib/motion/roles/transitions';

  interface Props {
    /** The screen-reader name and tooltip of the menu button (UI-015). */
    label: string;
    /** Whether the session has started, so there is something to pop out. */
    ready: boolean;
    onpopout: () => void;
  }

  const { label, ready, onpopout }: Props = $props();
  let open = $state(false);
  let control: HTMLButtonElement | undefined = $state();
  let menu: HTMLElement | undefined = $state();

  function dismiss(event: PointerEvent): void {
    if (open && event.target instanceof Node && !control?.contains(event.target) && !menu?.contains(event.target)) {
      open = false;
    }
  }

  function key(event: KeyboardEvent): void {
    if (event.key !== 'Escape') return;
    open = false;
    control?.focus();
  }

  function popOut(): void {
    open = false;
    onpopout();
  }

  $effect(() => {
    if (open) menu?.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
  });
</script>

<svelte:window onpointerdown={dismiss} />

<span class="anchor" data-no-drag>
  <button
    type="button"
    class="more"
    aria-label={label}
    title={label}
    aria-haspopup="menu"
    aria-expanded={open}
    bind:this={control}
    onclick={() => (open = !open)}
  >
    <Icon name="nf-md-dots_vertical" size="sm" />
  </button>
  {#if open}
    <div class="menu" role="menu" tabindex="-1" bind:this={menu} onkeydown={key} transition:popover={{ from: control }}>
      <button type="button" role="menuitem" disabled={!ready} onclick={popOut}>Pop out</button>
    </div>
  {/if}
</span>

<style>
  .anchor {
    position: relative;
    display: inline-flex;
  }

  button {
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    block-size: var(--target-min-desktop);
    border: 0;
    background: transparent;
    color: var(--text);
    font: inherit;
    cursor: default;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover:not(:disabled) {
    background: var(--surface-raised);
  }

  button:disabled {
    color: var(--text-muted);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }

  .more {
    inline-size: var(--target-min-desktop);
  }

  .menu {
    position: absolute;
    inset-block-start: 100%;
    inset-inline-start: 0;
    z-index: var(--layer-popover);
    display: flex;
    flex-direction: column;
    padding: var(--space-1);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    box-shadow: var(--elevation-2);
  }

  .menu button {
    justify-content: flex-start;
    padding: 0 var(--space-3);
    border-radius: var(--radius-sm);
    white-space: nowrap;
  }
</style>
