<script lang="ts">
  import { tick } from 'svelte';
  import { placeNear } from '$lib/devices/actions/placement';
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import { press } from '$lib/motion/roles/press';
  import { popover } from '$lib/motion/roles/transitions';
  import { nextIndex, type SplitChoice } from './split';

  interface Props {
    /** The main action's words, and its icon (UI-026). */
    label: string;
    icon?: IconName | undefined;
    /** The other ways to do it, in the menu behind the chevron. */
    choices: readonly SplitChoice[];
    /** Named for assistive technology: "More ways to connect to web-1". */
    menuLabel: string;
    tone?: 'primary' | 'secondary';
    onMain: (button: HTMLElement) => void;
    onChoose: (choice: SplitChoice, button: HTMLElement) => void;
  }

  const { label, icon, choices, menuLabel, tone = 'secondary', onMain, onChoose }: Props = $props();
  const id = $props.id();
  let open = $state(false);
  let root: HTMLElement | undefined;
  let chevron = $state<HTMLButtonElement>();
  let items: HTMLButtonElement[] = [];
  const place = placeNear(() => chevron);

  async function show(focus: 'first' | 'last'): Promise<void> {
    open = true;
    await tick();
    items[focus === 'first' ? 0 : items.length - 1]?.focus();
  }

  function close(): void {
    open = false;
    chevron?.focus();
  }

  function choose(choice: SplitChoice): void {
    open = false;
    if (chevron !== undefined) onChoose(choice, chevron);
  }

  function onChevronKey(event: KeyboardEvent): void {
    if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      void show(event.key === 'ArrowDown' ? 'first' : 'last');
    }
  }

  function onMenuKey(event: KeyboardEvent): void {
    if (event.key === 'Escape' || event.key === 'Tab') {
      if (event.key === 'Escape') event.preventDefault();
      close();
      return;
    }
    const current = items.findIndex((item) => item === document.activeElement);
    const next = nextIndex(event.key, current, items.length);
    if (next === null) return;
    event.preventDefault();
    items[next]?.focus();
  }

  /** A press outside closes the menu. */
  function onOutside(event: PointerEvent): void {
    if (open && root !== undefined && event.target instanceof Node && !root.contains(event.target)) open = false;
  }
</script>

<svelte:document onpointerdown={onOutside} />

<!-- A split button (UR-7, D16): the usual action, and a chevron for the other ways to do it. -->
<span class="split {tone}" bind:this={root}>
  <button type="button" class="main" tabindex="0" onclick={(event) => { onMain(event.currentTarget); }} {@attach press()}>
    {#if icon !== undefined}<Icon name={icon} size="sm" />{/if}{label}
  </button>
  <button
    type="button"
    class="chevron"
    tabindex="0"
    aria-label={menuLabel}
    aria-haspopup="menu"
    aria-expanded={open}
    aria-controls="{id}-menu"
    bind:this={chevron}
    onclick={() => {
      if (open) open = false;
      else void show('first');
    }}
    onkeydown={onChevronKey}
    {@attach press()}
  >
    <Icon name="carbon:chevron-down" size="sm" />
  </button>
  {#if open}
    <div class="menu" id="{id}-menu" role="menu" aria-label={menuLabel} tabindex="-1" onkeydown={onMenuKey} transition:popover={{ from: chevron }} {@attach place}>
      {#each choices as choice, index (choice.id)}
        <button type="button" role="menuitem" tabindex="-1" bind:this={items[index]} onclick={() => { choose(choice); }}>
          {#if choice.icon !== undefined}<Icon name={choice.icon} size="sm" />{/if}
          <span class="words">{choice.label}</span>
          {#if choice.shortcut !== undefined}<kbd>{choice.shortcut}</kbd>{/if}
        </button>
      {/each}
    </div>
  {/if}
</span>

<style>
  .split {
    display: inline-flex;
  }

  .main,
  .chevron {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    block-size: var(--control-md);
    border: 0;
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  .main {
    padding: 0 var(--space-3);
    border-start-start-radius: var(--radius-md);
    border-end-start-radius: var(--radius-md);
    white-space: nowrap;
  }

  .chevron {
    justify-content: center;
    inline-size: var(--control-sm);
    border-inline-start: var(--border-thin) solid var(--surface-sunken);
    border-start-end-radius: var(--radius-md);
    border-end-end-radius: var(--radius-md);
  }

  .main:hover,
  .chevron:hover {
    background: var(--surface-overlay);
  }

  .primary .main,
  .primary .chevron {
    background: var(--accent);
    color: var(--on-accent);
    font-weight: var(--weight-medium);
  }

  .primary .main:hover,
  .primary .chevron:hover {
    background: var(--accent-hover);
  }

  .main:focus-visible,
  .chevron:focus-visible,
  .menu button:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }

  .menu {
    position: fixed;
    z-index: var(--layer-popover);
    display: flex;
    flex-direction: column;
    min-inline-size: 12rem;
    margin-block-end: var(--space-1);
    padding: var(--space-1);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-overlay);
    box-shadow: var(--elevation-2);
  }

  .menu button {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--control-md);
    padding: 0 var(--space-2);
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text);
    font: inherit;
    text-align: start;
  }

  .menu button:hover,
  .menu button:focus-visible {
    background: var(--surface-hover);
  }

  .words {
    flex: 1 1 auto;
  }

  kbd {
    padding-inline-start: var(--space-4);
    color: var(--text-quiet);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
  }
</style>
