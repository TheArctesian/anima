<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import { popover } from '$lib/motion/roles/transitions';
  import { coverWhile } from '../overlay.svelte';
  import { placeInside } from './placement';

  interface Props {
    /** What the tip is about, for the button's name: "About <label>". */
    label: string;
    /** The tip: short text, a fingerprint, a list. Errors do not go here (UI-028). */
    children: Snippet;
  }

  /*
   * Never place a HelpTip over a live remote display (MAT-008). While it is open it covers the panes, so native web
   * views hide (WEB-005). WCAG 1.4.13: Escape dismisses it, the pointer can move onto it, and it stays until the
   * operator moves away, presses Escape or clicks elsewhere.
   */
  const { label, children }: Props = $props();
  const id = $props.id();
  let root: HTMLElement | undefined;
  let button = $state<HTMLButtonElement>();
  let pinned = $state(false);
  let hovered = $state(false);
  let focused = $state(false);
  /** Escape or a second click closed it; it stays closed until the operator comes back. */
  let dismissed = $state(false);
  const open = $derived(!dismissed && (pinned || hovered || focused));

  coverWhile(() => open);

  /** Click or Enter pins the tip open; the next one closes it. */
  function toggle(): void {
    dismissed = pinned;
    pinned = !pinned;
  }

  function onpointerenter(event: PointerEvent): void {
    if (event.pointerType === 'touch') return;
    hovered = true;
    dismissed = false;
  }

  function onpointerleave(): void {
    hovered = false;
  }

  function onfocusin(event: FocusEvent): void {
    if (!(event.target instanceof Element) || !event.target.matches(':focus-visible')) return;
    focused = true;
    dismissed = false;
  }

  /** Focus moving to something else inside the tip keeps it open; moving away closes it. */
  function onfocusout(event: FocusEvent): void {
    if (event.relatedTarget instanceof Node && root?.contains(event.relatedTarget)) return;
    focused = false;
    pinned = false;
  }

  function onkeydown(event: KeyboardEvent): void {
    if (event.key !== 'Escape' || !open) return;
    // Focus first: its focusin would otherwise undo the dismissal.
    button?.focus();
    dismissed = true;
    pinned = false;
  }

  function onpointerdown(event: PointerEvent): void {
    if (event.target instanceof Node && root?.contains(event.target)) return;
    pinned = false;
    focused = false;
  }

  /** The tip reacts to the pointer and to focus anywhere inside it, so the listeners sit on its root, not on a control. */
  function watch(element: HTMLElement): () => void {
    element.addEventListener('pointerenter', onpointerenter);
    element.addEventListener('pointerleave', onpointerleave);
    element.addEventListener('focusin', onfocusin);
    element.addEventListener('focusout', onfocusout);
    return () => {
      element.removeEventListener('pointerenter', onpointerenter);
      element.removeEventListener('pointerleave', onpointerleave);
      element.removeEventListener('focusin', onfocusin);
      element.removeEventListener('focusout', onfocusout);
    };
  }

  /** Keeps the panel under the button, inside the window, through scroll and resize. */
  function place(panel: HTMLElement): () => void {
    const reposition = () => {
      if (button === undefined) return;
      const size = { width: panel.offsetWidth, height: panel.offsetHeight };
      const viewport = { width: document.documentElement.clientWidth, height: document.documentElement.clientHeight };
      const at = placeInside(button.getBoundingClientRect(), size, viewport);
      panel.style.left = `${String(at.left)}px`;
      panel.style.top = `${String(at.top)}px`;
    };
    reposition();
    addEventListener('scroll', reposition, true);
    addEventListener('resize', reposition);
    return () => {
      removeEventListener('scroll', reposition, true);
      removeEventListener('resize', reposition);
    };
  }
</script>

<svelte:document {onkeydown} {onpointerdown} />

<span class="helptip" bind:this={root} {@attach watch}>
  <!-- tabindex="0": WebKit skips plain buttons when Tab is pressed unless they are focusable (NAV-005). -->
  <button type="button" tabindex="0" bind:this={button} aria-label="About {label}" aria-expanded={open} aria-controls="{id}-tip" onclick={toggle}>
    <Icon name="nf-md-information_outline" size="sm" />
  </button>
  {#if open}
    <div class="anchor" transition:popover={{ from: button }} {@attach place}>
      <div class="panel" id="{id}-tip" role="status">{@render children()}</div>
    </div>
  {/if}
</span>

<style>
  .helptip {
    display: inline-flex;
    vertical-align: middle;
  }

  /* 16 px glyph in a 24 px target (UI-013). */
  button {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    inline-size: var(--target-min-desktop);
    block-size: var(--target-min-desktop);
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius-pill);
    color: var(--text-secondary);
    font: inherit;
    transition: color var(--duration-fast) var(--ease-enter);
  }

  button:hover,
  button[aria-expanded='true'] {
    color: var(--text);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-thin);
  }

  /* Fixed, so no scrolling parent clips it. Its padding is a gap the pointer can cross without closing the tip. */
  .anchor {
    position: fixed;
    z-index: var(--layer-popover);
    padding: var(--space-2);
  }

  .panel {
    box-sizing: border-box;
    inline-size: max-content;
    max-inline-size: 18rem;
    padding: var(--space-3);
    background: var(--surface-overlay);
    border-radius: var(--radius-md);
    box-shadow: var(--elevation-2);
    color: var(--text);
    font-size: var(--text-sm);
    font-weight: var(--weight-regular);
    line-height: var(--leading-normal);
    text-align: start;
    white-space: normal;
  }

  .panel :global(p),
  .panel :global(ul) {
    margin: 0;
  }

  .panel :global(ul) {
    padding-inline-start: var(--space-4);
  }

  .panel :global(code) {
    font-family: var(--font-mono);
    font-variant-ligatures: none;
    overflow-wrap: anywhere;
  }
</style>
