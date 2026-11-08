<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import { press } from '$lib/motion/roles/press';
  import { popover } from '$lib/motion/roles/transitions';
  import { menuLabel, shiftInsideViewport, type MenuOption } from './toolbar';

  interface Props {
    label: string;
    icon: IconName;
    options: readonly MenuOption[];
    /** The chosen values. */
    chosen: readonly string[];
    /** Checkboxes choose many; radios choose one. */
    kind: 'checkbox' | 'radio';
    /** Shown in place of the options when there are none. */
    empty?: string;
    onChoose: (value: string) => void;
  }

  const { label, icon, options, chosen, kind, empty = '', onChoose }: Props = $props();
  const id = $props.id();
  let open = $state(false);
  let root: HTMLElement | undefined;
  let button = $state<HTMLButtonElement>();

  /** Moves the panel left or right until it is inside the window; the control it grows from keeps the origin (MOT-005). */
  function keepInside(panel: HTMLElement): void {
    const box = panel.getBoundingClientRect();
    // One panel padding from the window edge.
    const margin = Number.parseFloat(getComputedStyle(panel).paddingInlineStart);
    const shift = shiftInsideViewport(box.left, box.width, document.documentElement.clientWidth, margin);
    panel.style.marginInlineStart = `${String(shift)}px`;
  }

  function close(): void {
    open = false;
    button?.focus();
  }

  /** Escape closes the open menu wherever focus is: WebKit does not focus a button that a mouse click opened. */
  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && open) close();
  }

  /** A press outside closes the menu; focus goes back to the control only when it was inside the menu. */
  function onOutside(event: PointerEvent): void {
    if (!open || root === undefined || (event.target instanceof Node && root.contains(event.target))) return;
    const hadFocus = root.contains(document.activeElement);
    open = false;
    if (hadFocus) button?.focus();
  }
</script>

<svelte:document onpointerdown={onOutside} onkeydown={onKeydown} />

<div class="menu" bind:this={root}>
  <!-- tabindex="0": WebKit skips plain buttons and form controls when Tab is pressed unless they are focusable (NAV-005). -->
  <button
    type="button"
    class="control"
    tabindex="0"
    bind:this={button}
    aria-expanded={open}
    aria-controls="{id}-panel"
    onclick={() => {
      open = !open;
    }}
    {@attach press()}
  >
    <Icon name={icon} size="sm" />
    <span>{kind === 'checkbox' ? menuLabel(label, chosen.length) : label}</span>
    <Icon name="nf-md-chevron_down" size="sm" />
  </button>
  {#if open}
    <div class="panel" id="{id}-panel" role="group" aria-label={label} transition:popover={{ from: button }} {@attach keepInside}>
      {#each options as option (option.value)}
        <label class="option">
          <input
            type={kind}
            tabindex="0"
            name={kind === 'radio' ? id : undefined}
            checked={chosen.includes(option.value)}
            onchange={() => {
              onChoose(option.value);
            }}
          />
          <span>{option.label}</span>
        </label>
      {:else}
        <p class="none">{empty}</p>
      {/each}
    </div>
  {/if}
</div>

<style>
  .menu {
    position: relative;
  }

  .control {
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    block-size: calc(var(--target-min-desktop) + var(--space-2));
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  .control:hover {
    background: var(--surface-raised);
  }

  .control:active {
    transition-duration: var(--duration-press);
    background: var(--surface-hover);
  }

  input {
    accent-color: var(--accent);
  }

  .control:focus-visible,
  input:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .panel {
    position: absolute;
    inset-block-start: 100%;
    inset-inline-start: 0;
    z-index: var(--layer-popover);
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    min-inline-size: calc(var(--space-16) * 3);
    max-inline-size: calc(100vw - var(--space-4) * 2);
    max-block-size: calc(var(--space-16) * 5);
    margin-block-start: var(--space-1);
    padding: var(--space-2);
    overflow-y: auto;
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--material-overlay-bg);
    backdrop-filter: blur(var(--material-overlay-blur)) saturate(var(--material-overlay-saturate));
    box-shadow: var(--elevation-2);
  }

  .option {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-2);
    border-radius: var(--radius-sm);
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  .option:hover {
    background: var(--surface-raised);
  }

  .none {
    margin: 0;
    padding: var(--space-2);
    color: var(--text);
  }
</style>
