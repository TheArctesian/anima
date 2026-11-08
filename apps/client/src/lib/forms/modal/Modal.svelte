<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import { coverWhile } from '$lib/overlay/overlay.svelte';
  import { fade, modal } from '$lib/motion/roles/transitions';
  import { offerFooter } from './footer';
  import { initialFocusIn, trapTab } from './focus';
  import { takeOpener, watchClicks } from './opener';
  import { openDialog, type OpenDialog } from './stack';

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
    /** A warning the operator must read: the surface is an `alertdialog` with a warning edge and icon. */
    warning?: boolean;
    /** The id of the element that describes the dialog, for assistive technology. */
    describedBy?: string;
    /** False when a click on the scrim must not close the dialog; Escape still calls `onClose`. */
    closeOnScrim?: boolean;
    /** Where focus goes when the dialog closes; the opener when absent or when it gives no element. */
    returnTo?: () => HTMLElement | undefined;
    /** The surface's width in steps of `--space-16` (a `content` surface is 8 wide). */
    width?: number;
    /** Lowers the surface by `--space-16` from the top edge, for a warning that sits over the page's own controls. */
    lowered?: boolean;
    children: Snippet;
  }

  const { open, title, onClose, size = 'content', warning = false, describedBy, closeOnScrim = true, returnTo, width = 8, lowered = false, children }: Props = $props();
  const titleId = $props.id();
  coverWhile(() => open);

  /**
   * A form's actions, shown below the body by the form inside (see `footer.ts`). Each showing is its own
   * entry, so a form that is replaced takes out its own actions and never those of the form after it.
   * Raw, so `shown` is the entry itself; a deep proxy never equals its value.
   */
  let shown = $state.raw<{ readonly actions: Snippet }>();
  /** The same entry, kept plain: a teardown reads `$state` as it was before, so it would find its own entry still shown. */
  let latest: { readonly actions: Snippet } | undefined;
  offerFooter({
    show: (actions) => {
      const entry = { actions };
      latest = entry;
      shown = entry;
      return () => {
        if (latest !== entry) return;
        latest = undefined;
        shown = undefined;
      };
    },
  });

  // The click that opens the dialog comes before `open`, so clicks are watched while it is closed too.
  $effect(watchClicks);

  let surface: HTMLElement | undefined = $state();
  let stacked: OpenDialog | undefined;

  /**
   * The opener is read before the dialog is in the DOM. The dialog leaves the stack as soon as `open` is
   * false, even while it fades out, and only then does focus go back to the opener.
   */
  $effect.pre(() => {
    if (!open) return;
    const entry = openDialog();
    stacked = entry;
    const opener = takeOpener(document.activeElement);
    return () => {
      entry.close();
      (returnTo?.() ?? opener)?.focus();
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

  /**
   * Focus starts inside the dialog. While it is the topmost dialog, keys pressed outside it (on the body,
   * after a click that took the focus away) are handled here too, and focus that lands outside goes back in.
   */
  $effect(() => {
    const dialog = surface;
    const entry = stacked;
    if (!open || dialog === undefined || entry === undefined) return;
    initialFocusIn(dialog).focus();
    const outside = (event: KeyboardEvent) => {
      // Svelte delegates onkeydown to the document: a key a dialog's surface already handled is not handled twice.
      if (event.defaultPrevented) return;
      if (!dialog.contains(event.target as Node | null) && entry.isTopmost()) keydown(event, dialog);
    };
    const pullBack = (event: FocusEvent) => {
      if (!dialog.contains(event.target as Node | null) && entry.isTopmost()) initialFocusIn(dialog).focus();
    };
    document.addEventListener('keydown', outside);
    document.addEventListener('focusin', pullBack);
    return () => {
      document.removeEventListener('keydown', outside);
      document.removeEventListener('focusin', pullBack);
    };
  });

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
    <div class="scrim" transition:fade|global>
      <div class="shade"></div>
    </div>
    <div
      class="frame"
      role="presentation"
      class:lowered
      onmousedown={(event) => {
        // A press on the scrim keeps the focus where it is, so Tab and Escape still reach the dialog.
        if (event.target === event.currentTarget) event.preventDefault();
      }}
      onpointerdown={(event) => {
        if (closeOnScrim && event.target === event.currentTarget) onClose();
      }}
    >
      <div
        class="surface"
        class:form={size === 'form'}
        class:warning
        role={warning ? 'alertdialog' : 'dialog'}
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={describedBy}
        tabindex="-1"
        transition:modal|global
        bind:this={surface}
        style:--modal-width={width}
        onkeydown={(event) => {
          keydown(event, event.currentTarget);
        }}
      >
        <h2 id={titleId}>{#if warning}<Icon name="nf-md-alert" />{/if}{title}</h2>
        <div class="body">
          {@render children()}
        </div>
        {#if shown}
          <div class="footer">{@render shown.actions()}</div>
        {/if}
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

  .lowered {
    padding-block-start: var(--space-16);
  }

  .surface {
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    inline-size: min(100%, calc(var(--space-16) * var(--modal-width)));
    max-block-size: 100%;
    padding: var(--space-5);
    background: var(--surface-raised);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--elevation-3);
    color: var(--text);
  }

  .warning {
    border-block-start: var(--border-focus) solid var(--status-warning);
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

  /* Below the body and outside it, so the actions never scroll, never move and never cover a field. */
  .footer {
    flex: none;
  }

  /* A scrollbar that comes and goes as sections open does not change the width of the fields. */
  .form .body {
    scrollbar-gutter: stable;
  }

  .surface:focus {
    outline: none;
  }

  h2 {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    font-size: var(--text-lg);
    font-weight: var(--weight-bold);
    line-height: var(--leading-tight);
  }

  h2 :global(.icon) {
    color: var(--status-warning);
  }
</style>
