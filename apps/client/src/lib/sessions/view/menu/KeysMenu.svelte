<script lang="ts" generics="Entry extends MenuEntry">
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { popover } from '$lib/motion/roles/transitions';
  import { menuAlignment, menuFocus, menuPlacement, type MenuAlignment, type MenuEntry, type MenuPlacement } from './menu';

  interface Props {
    /** No session to send to (not ready, or ended). */
    disabled: boolean;
    /** The key combinations, in menu order (RA-006). */
    entries: readonly Entry[];
    onkeys: (entry: Entry) => void;
    onpaste: () => void;
    /** Why "Paste" is off (RA-033); the item then shows it and sends nothing. */
    pasteOff?: string | undefined;
  }

  const { disabled, entries, onkeys, onpaste, pasteOff }: Props = $props();
  const menuId = $props.id();
  let open = $state(false);
  let trigger: HTMLButtonElement | undefined = $state();
  let menu: HTMLElement | undefined = $state();
  let placement: MenuPlacement | undefined = $state();
  let alignment: MenuAlignment | undefined = $state();

  function items(): HTMLElement[] {
    return menu === undefined ? [] : [...menu.querySelectorAll<HTMLElement>('[role="menuitem"]')];
  }

  function show(at: 'first' | 'last') {
    open = true;
    placement = undefined;
    alignment = undefined;
    // The menu exists after this update: place it on the side with room, then focus its first (or last) item.
    queueMicrotask(() => {
      if (trigger !== undefined && menu !== undefined) {
        const control = trigger.getBoundingClientRect();
        placement = menuPlacement(control, window.innerHeight, menu.scrollHeight);
        alignment = menuAlignment(control, window.innerWidth, menu.scrollWidth);
      }
      const all = items();
      (at === 'first' ? all[0] : all.at(-1))?.focus();
    });
  }

  function hide(refocus: boolean) {
    open = false;
    if (refocus) trigger?.focus();
  }

  function choose(action: () => void) {
    hide(true);
    action();
  }

  function triggerKey(event: KeyboardEvent) {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      show(event.key === 'ArrowDown' ? 'first' : 'last');
    }
  }

  function menuKey(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      hide(true);
    } else if (event.key === 'Tab') {
      hide(false);
    } else {
      const next = menuFocus(event.key, items(), document.activeElement);
      if (next === undefined) return;
      event.preventDefault();
      next.focus();
    }
  }

  // A press outside the menu and its control closes it.
  $effect(() => {
    if (!open) return;
    const outside = (event: PointerEvent) => {
      const target = event.target as Node | null;
      if (target !== null && !menu?.contains(target) && !trigger?.contains(target)) hide(false);
    };
    document.addEventListener('pointerdown', outside, true);
    return () => {
      document.removeEventListener('pointerdown', outside, true);
    };
  });

  $effect(() => {
    if (disabled) open = false;
  });
</script>

<div class="keys">
  <button
    type="button"
    class="trigger"
    bind:this={trigger}
    {disabled}
    aria-haspopup="menu"
    aria-expanded={open}
    aria-controls={menuId}
    title="Send a key combination"
    onclick={() => (open ? hide(false) : show('first'))}
    onkeydown={triggerKey}
    {@attach press()}
  >
    <Icon name="nf-md-key" />
    <span>Keys</span>
  </button>
  {#if open}
    <!-- Solid, not a translucent material: the menu lies over the live display (MAT-008). -->
    <div
      class="menu"
      id={menuId}
      role="menu"
      tabindex="-1"
      aria-label="Keys"
      bind:this={menu}
      data-side={placement?.side ?? 'below'}
      data-align={alignment?.align ?? 'start'}
      style:max-block-size={placement === undefined ? undefined : `${String(placement.maxHeight)}px`}
      style:--menu-room={alignment === undefined ? undefined : `${String(alignment.maxWidth)}px`}
      onkeydown={menuKey}
      transition:popover={{ from: trigger }}
    >
      {#each entries as combination (combination.id)}
        <button type="button" role="menuitem" tabindex="-1" onclick={() => choose(() => onkeys(combination))}>
          <span class="label">{combination.label}</span>
          <span class="hint">{combination.hint}</span>
        </button>
      {/each}
      <hr />
      {#if pasteOff === undefined}
        <button type="button" role="menuitem" tabindex="-1" onclick={() => choose(onpaste)}>
          <span class="label">Paste</span>
          <span class="hint">Send the clipboard</span>
        </button>
      {:else}
        <!-- Focusable but inert, so the reason is read with the item (WAI-ARIA menu pattern). -->
        <button type="button" role="menuitem" tabindex="-1" aria-disabled="true" data-off>
          <span class="label">Paste</span>
          <span class="hint">{pasteOff}</span>
        </button>
      {/if}
    </div>
  {/if}
</div>

<style>
  .keys {
    flex: none;
    position: relative;
  }

  .trigger {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  .trigger:disabled {
    color: var(--text-muted);
  }

  .trigger:focus-visible,
  .menu button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }

  .menu {
    position: absolute;
    inset-block-start: calc(100% + var(--space-1));
    inset-inline-start: 0;
    box-sizing: border-box;
    overflow-y: auto;
    z-index: var(--layer-popover);
    display: flex;
    flex-direction: column;
    min-inline-size: min(calc(var(--space-16) * 4), var(--menu-room, 100vw));
    max-inline-size: var(--menu-room, none);
    padding: var(--space-1);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    box-shadow: var(--elevation-2);
  }

  .menu[data-align='end'] {
    inset-inline-start: auto;
    inset-inline-end: 0;
  }

  .menu[data-side='above'] {
    inset-block-start: auto;
    inset-block-end: calc(100% + var(--space-1));
  }

  .menu button {
    display: flex;
    flex: none;
    align-items: center;
    justify-content: space-between;
    white-space: nowrap;
    gap: var(--space-4);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: start;
  }

  .menu button:hover,
  .menu button:focus {
    background: var(--border);
  }

  .label {
    font-family: var(--font-mono);
  }

  .hint {
    color: var(--text);
    font-size: var(--text-sm);
  }

  .menu button[data-off] {
    color: var(--text-muted);
  }

  .menu button[data-off] .hint {
    color: var(--text-muted);
    white-space: normal;
  }

  hr {
    inline-size: 100%;
    margin: var(--space-1) 0;
    border: 0;
    border-top: var(--border-thin) solid var(--border);
  }
</style>
