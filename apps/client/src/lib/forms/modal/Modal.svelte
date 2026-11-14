<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import { press } from '$lib/motion/roles/press';
  import { coverWhile } from '$lib/overlay/overlay.svelte';
  import { fade, modal } from '$lib/motion/roles/transitions';
  import { edgeFade } from '$lib/scroll/edges';
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
    /** A size preset: `sm` for a confirmation, `md` for a form, `lg` for a wide task. It sets the width. */
    preset?: 'sm' | 'md' | 'lg';
    /** The actions below the body, secondary first and primary last; wins over a form's own footer. */
    footer?: Snippet;
    /** Lowers the surface by `--space-16` from the top edge, for a warning that sits over the page's own controls. */
    lowered?: boolean;
    /** An icon before the title, for a dialog that is a task (Add device); a warning's icon wins over it. */
    icon?: IconName;
    /** Shows a Close button at the end of the title row; it calls `onClose`, as Escape does. */
    dismissible?: boolean;
    /** Solid, not frosted: for a dialog that can open over a live remote display (MAT-008). */
    solid?: boolean;
    children: Snippet;
  }

  const { open, title, onClose, size = 'content', warning = false, describedBy, closeOnScrim = true, returnTo, width = 8, preset, footer, lowered = false, icon, dismissible = false, solid = false, children }: Props = $props();
  const PRESET_WIDTH = { sm: 6, md: 8, lg: 12 } as const;
  const span = $derived(preset === undefined ? width : PRESET_WIDTH[preset]);
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
        class="surface frosted-host"
        class:form={size === 'form'}
        class:warning
        class:dismissible
        class:marked={icon !== undefined && !warning}
        role={warning ? 'alertdialog' : 'dialog'}
        aria-modal="true"
        aria-labelledby={titleId}
        aria-describedby={describedBy}
        tabindex="-1"
        transition:modal|global
        bind:this={surface}
        style:--modal-width={span}
        onkeydown={(event) => {
          keydown(event, event.currentTarget);
        }}
      >
        <!-- The glass sits behind the content, not on the surface: a backdrop filter would make the surface the containing block of a fixed HelpTip (MAT-010). -->
        <div class="glass-layer material-overlay" data-solid={solid || undefined} aria-hidden="true"></div>
        <h2 id={titleId}>{#if warning}<Icon name="nf-md-alert" />{/if}{title}</h2>
        {#if icon !== undefined && !warning}
          <!-- Beside the title, not in it: the heading's text stays the title alone. -->
          <span class="mark"><Icon name={icon} /></span>
        {/if}
        {#if dismissible}
          <!-- Not the first stop: focus still starts in the body (see `initialFocusIn`). UI-015: a label and a tooltip. -->
          <button type="button" class="close" data-dismiss aria-label="Close" title="Close" onclick={onClose} {@attach press()}>
            <Icon name="nf-md-close" />
          </button>
        {/if}
        <div class="body" {@attach edgeFade}>
          {@render children()}
        </div>
        {#if footer}
          <div class="footer actions">{@render footer()}</div>
        {:else if shown}
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
    position: relative;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    inline-size: min(100%, calc(var(--space-16) * var(--modal-width)));
    max-block-size: 100%;
    padding: var(--space-5);
    border-radius: var(--radius-lg);
    box-shadow: var(--elevation-3);
    color: var(--text);
    /* Keeps the glass layer's negative stacking inside the surface. */
    isolation: isolate;
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

  /* A short fade where the body scrolls, not a line (MAT-003, MAT-004). */
  .body:global([data-more-below]) {
    mask-image: linear-gradient(to bottom, var(--text) calc(100% - var(--space-6)), transparent);
  }

  .body:global([data-more-above]) {
    mask-image: linear-gradient(to top, var(--text) calc(100% - var(--space-6)), transparent);
  }

  .body:global([data-more-above][data-more-below]) {
    mask-image: linear-gradient(to bottom, transparent, var(--text) var(--space-6), var(--text) calc(100% - var(--space-6)), transparent);
  }

  /* Below the body and outside it, so the actions never scroll, never move and never cover a field. */
  .footer {
    flex: none;
  }

  /* The secondary action first, the primary last at the end of the row. */
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
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
    font-weight: var(--weight-medium);
    line-height: var(--leading-tight);
  }

  h2 :global(.icon) {
    color: var(--status-warning);
  }

  /*
   * A task's title row: as tall as the Close button, with the task's icon in the padding at its start and
   * the Close button in the padding at its end, both laid over the surface's padding box.
   */
  .marked h2,
  .dismissible h2 {
    min-block-size: var(--space-8);
  }

  .marked h2 {
    padding-inline-start: calc(var(--icon-md) + var(--space-3));
  }

  .dismissible h2 {
    padding-inline-end: var(--space-10);
  }

  .mark {
    position: absolute;
    inset-block-start: var(--space-5);
    inset-inline-start: var(--space-5);
    display: grid;
    place-items: center;
    block-size: var(--space-8);
    color: var(--accent);
  }

  .close {
    position: absolute;
    inset-block-start: var(--space-5);
    inset-inline-end: var(--space-4);
    display: grid;
    place-items: center;
    box-sizing: border-box;
    inline-size: var(--space-8);
    block-size: var(--space-8);
    padding: 0;
    background: none;
    border: 0;
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  .close:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  .close:active {
    transition-duration: var(--duration-press);
    background: var(--accent-tint);
  }

  .close:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }
</style>
