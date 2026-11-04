<script lang="ts">
  import type { Snippet } from 'svelte';
  import { crossFade, modal } from '$lib/motion/roles/transitions';
  import { initialFocusIn, trapTab } from './focus';

  interface Props {
    open: boolean;
    title: string;
    /** Called for Escape and for a click on the scrim; the owner sets `open` to false. */
    onClose: () => void;
    children: Snippet;
  }

  const { open, title, onClose, children }: Props = $props();
  const titleId = $props.id();

  // The opener is read before the dialog is in the DOM; focus goes back to it as soon as `open` is false.
  $effect.pre(() => {
    if (!open) return;
    const opener = document.activeElement;
    return () => {
      if (opener instanceof HTMLElement) opener.focus();
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
</script>

{#if open}
  <div class="scrim" transition:crossFade>
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
      {@render children()}
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
    background: var(--bg);
    opacity: 0.6;
  }

  .frame {
    display: grid;
    place-items: center;
    padding: var(--space-4);
  }

  .surface {
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    inline-size: min(100%, calc(var(--space-16) * 8));
    max-block-size: 100%;
    overflow-y: auto;
    padding: var(--space-5);
    background: var(--material-overlay-bg);
    backdrop-filter: blur(var(--material-overlay-blur)) saturate(var(--material-overlay-saturate));
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-lg);
    box-shadow: var(--elevation-3);
    color: var(--text);
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
