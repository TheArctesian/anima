<script lang="ts">
  import type { Snippet } from 'svelte';
  import { fade, modal } from '$lib/motion/roles/transitions';
  import { initialFocusIn, trapTab } from './focus';
  import { takeOpener, watchClicks } from './opener';

  interface Props {
    open: boolean;
    title: string;
    /** Called for Escape and for a click on the scrim; the owner sets `open` to false. */
    onClose: () => void;
    /**
     * `content` sizes the surface to what it holds. `form` gives it a fixed block size, for a form whose
     * sections open and close: the surface's edge never moves and the body scrolls inside it.
     */
    size?: 'content' | 'form';
    children: Snippet;
  }

  const { open, title, onClose, size = 'content', children }: Props = $props();
  const titleId = $props.id();

  // The click that opens the dialog comes before `open`, so clicks are watched while it is closed too.
  $effect(watchClicks);

  // The opener is read before the dialog is in the DOM; focus goes back to it as soon as `open` is false.
  $effect.pre(() => {
    if (!open) return;
    const opener = takeOpener(document.activeElement);
    return () => {
      opener?.focus();
    };
  });

  function keydown(event: KeyboardEvent, dialog: HTMLElement): void {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      onClose();
    } else if (trapTab(event, dialog)) {
      event.stopPropagation();
    }
  }

  function enter(dialog: HTMLElement): void {
    initialFocusIn(dialog).focus();
  }

  /**
   * Moves the dialog's root to `body`, so that a dialog opened from inside another one is not clipped by the
   * outer surface (which is a containing block for fixed children). The root is the block's only node.
   */
  function toBody(root: HTMLElement): () => void {
    const focused = document.activeElement;
    document.body.append(root);
    // Moving a node drops focus from it.
    if (focused instanceof HTMLElement && root.contains(focused)) focused.focus();
    return () => {
      root.remove();
    };
  }
</script>

{#if open}
  <div class="portal" {@attach toBody}>
    <div class="scrim" transition:fade>
      <div class="shade"></div>
    </div>
    <div
      class="frame"
      role="presentation"
      onpointerdown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        class="surface"
        class:form={size === 'form'}
        role="dialog"
        aria-modal="true"
        aria-labelledby={titleId}
        tabindex="-1"
        transition:modal
        {@attach enter}
        onkeydown={(event) => {
          keydown(event, event.currentTarget);
        }}
      >
        <h2 id={titleId}>{title}</h2>
        <div class="body">
          {@render children()}
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .scrim,
  .frame {
    position: fixed;
    inset: 0;
    z-index: var(--layer-modal);
  }

  .shade {
    block-size: 100%;
    background: var(--scrim);
  }

  /*
   * The surface's top is fixed, so a section that opens inside it never moves the control the operator
   * clicked (MOT-007). A `content` surface grows downward to the window's height; a `form` surface has a
   * fixed height, so its edge never moves while a section opens or closes. Past that the body scrolls.
   */
  .frame {
    display: grid;
    /* One row of the window's height, not of the surface's content, so `100%` below is the window. */
    grid-template-rows: minmax(0, 1fr);
    justify-items: center;
    align-items: start;
    padding: var(--space-4);
  }

  .surface {
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    inline-size: min(100%, calc(var(--space-16) * 8));
    max-block-size: 100%;
    padding: var(--space-5);
    background: var(--surface-raised);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--elevation-3);
    color: var(--text);
  }

  .form {
    block-size: min(100%, calc(var(--space-16) * 12));
  }

  /*
   * The scroll container, so a section's FLIP stays inside it (5.7.3 Disclosure). It reaches into the
   * surface's padding, so focus rings at its edges are not clipped and the scrollbar sits at the edge.
   * Scroll anchoring is off: the content below a section moves by FLIP alone, and the browser does not
   * shift the scroll position to follow it, which would move the control the operator clicked.
   */
  .body {
    display: flex;
    flex: 1 1 auto;
    flex-direction: column;
    gap: var(--space-3);
    min-block-size: 0;
    margin: calc(-1 * var(--space-1)) calc(-1 * var(--space-5));
    padding: var(--space-1) var(--space-5);
    overflow-y: auto;
    overflow-anchor: none;
  }

  /* A scrollbar that comes and goes as sections open does not change the width of the fields. */
  .form .body {
    scrollbar-gutter: stable;
  }

  .surface:focus {
    outline: none;
  }

  h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: var(--weight-bold);
    line-height: var(--leading-tight);
  }
</style>
