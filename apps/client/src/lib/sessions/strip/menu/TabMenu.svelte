<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { popover } from '$lib/motion/roles/transitions';
  import { coverWhile } from '$lib/overlay/overlay.svelte';
  import { keepFocus, portal, under } from './anchor';

  interface Props {
    /** The tooltip and name of the menu button (UI-015). */
    label: string;
    /** Whether the session has started, so there is something to pop out. */
    ready: boolean;
    /** Whether the menu shows; the tab opens it from the keyboard (Shift+F10, ContextMenu). */
    open?: boolean;
    onpopout: () => void;
    /** Called when the menu closes from the keyboard, so focus can go back to the tab. */
    onreturn: () => void;
  }

  let { label, ready, open = $bindable(false), onpopout, onreturn }: Props = $props();
  let control: HTMLButtonElement | undefined = $state();
  let menu: HTMLElement | undefined = $state();
  coverWhile(() => open);

  function dismiss(event: Event): void {
    if (open && event.target instanceof Node && !control?.contains(event.target) && !menu?.contains(event.target)) {
      open = false;
    }
  }

  /** Escape works wherever focus is while the menu is open, even with every item disabled. */
  function key(event: KeyboardEvent): void {
    if (!open || event.key !== 'Escape') return;
    event.preventDefault();
    open = false;
    onreturn();
  }

  /** Tab out of the menu closes it; a press elsewhere is `dismiss`'s, and on the control it toggles. */
  function leave(event: FocusEvent): void {
    const to = event.relatedTarget;
    if (to instanceof Node && !menu?.contains(to) && !control?.contains(to)) open = false;
  }

  function popOut(): void {
    if (!ready) return;
    open = false;
    onpopout();
  }

  /** A press on the control toggles the menu; closing it this way gives focus back to the tab too. */
  function toggle(): void {
    open = !open;
    if (!open) onreturn();
  }

  $effect(() => {
    if (open) menu?.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
  });
</script>

<svelte:window onpointerdown={dismiss} onkeydown={key} onresize={() => (open = false)} />
<svelte:document onscrollcapture={() => (open = false)} />

<!-- Not a Tab stop and hidden from the tablist, which owns only tabs: the keyboard opens the menu from the tab. -->
<button
  type="button"
  class="more"
  title={label}
  aria-label={label}
  aria-hidden="true"
  tabindex="-1"
  aria-haspopup="menu"
  aria-expanded={open}
  data-no-drag
  bind:this={control}
  onpointerdown={keepFocus}
  onmousedown={keepFocus}
  onfocus={onreturn}
  onclick={toggle}
  {@attach press()}
>
  <Icon name="nf-md-dots_vertical" size="sm" />
</button>
{#if open}
  <div
    class="menu"
    role="menu"
    tabindex="-1"
    aria-label={label}
    bind:this={menu}
    onfocusout={leave}
    transition:popover={{ from: control }}
    {@attach portal}
    {@attach under(() => control)}
  >
    <button type="button" role="menuitem" aria-disabled={!ready} onclick={popOut}>Pop out</button>
  </div>
{/if}

<style>
  button {
    box-sizing: border-box;
    display: inline-flex;
    flex: none;
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

  button:hover:not([aria-disabled='true']) {
    background: var(--surface-overlay);
  }

  /* No disabled colour token exists; muted text is the nearest (Section 5.3). */
  button[aria-disabled='true'] {
    color: var(--text-secondary);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }

  .more {
    inline-size: var(--target-min-desktop);
  }

  .menu {
    position: fixed;
    z-index: var(--layer-popover);
    display: flex;
    flex-direction: column;
    padding: var(--space-1);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-overlay);
    box-shadow: var(--elevation-2);
    font-family: var(--font-ui);
  }

  .menu button {
    justify-content: flex-start;
    padding: 0 var(--space-3);
    border-radius: var(--radius-sm);
    white-space: nowrap;
  }
</style>
