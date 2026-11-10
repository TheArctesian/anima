<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import type { ActionState } from './feedback/feedback';

  interface Props extends Omit<HTMLButtonAttributes, 'class' | 'children'> {
    /** `danger` is for the button that destroys something. */
    tone?: 'plain' | 'primary' | 'danger';
    children: Snippet;
    /** For a button that starts hub work: its label gives way to `pendingLabel`, then to a check and `doneLabel`. */
    feedback?: ActionState;
    pendingLabel?: string;
    doneLabel?: string;
  }

  const { tone = 'plain', type = 'button', children, feedback, pendingLabel = 'Saving', doneLabel = 'Saved', ...rest }: Props = $props();
</script>

<button class={tone} {type} aria-busy={feedback === 'pending' || undefined} {...rest} {@attach press()}>
  {#if feedback === undefined}
    {@render children()}
  {:else}
    <!-- All three labels share one cell, so the button keeps its width; each fades in place (MOT-007, MOT-008). -->
    <span class="labels">
      <span class="label" class:on={feedback === 'idle'} aria-hidden={feedback !== 'idle'}>{@render children()}</span>
      <span class="label waiting" class:on={feedback === 'pending'} aria-hidden={feedback !== 'pending'}>{pendingLabel}</span>
      <span class="label done" class:on={feedback === 'done'} aria-hidden={feedback !== 'done'}><Icon name="nf-md-check" size="sm" />{doneLabel}</span>
    </span>
  {/if}
</button>

<style>
  button {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-4);
    background: var(--surface-raised);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  .labels {
    display: inline-grid;
  }

  .label {
    grid-area: 1 / 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-1);
    opacity: 0;
    transition: opacity var(--duration-base) var(--ease-enter);
  }

  .label.on {
    opacity: 1;
  }

  /* No spinner: the word says the hub is working (5.7.3, Feedback). */
  .waiting {
    color: var(--text-muted);
  }

  .done :global(.icon) {
    color: var(--status-up);
  }

  /* On the accent fill the label keeps the button's own colour, for contrast (UI-004). */
  .primary :is(.waiting, .done :global(.icon)) {
    color: inherit;
  }

  button:hover:not(:disabled, [aria-disabled='true'], .primary) {
    background: var(--surface-hover);
  }

  button:active:not(:disabled, [aria-disabled='true'], .primary) {
    transition-duration: var(--duration-press);
    background: var(--accent-tint);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  button:disabled,
  button[aria-disabled='true'] {
    color: var(--text-muted);
  }

  /* UI-004: accent with a --bg label, accent-deep while pressed, as WakeButton. */
  .primary {
    background: var(--accent);
    border-color: var(--accent);
    color: var(--bg);
  }

  .primary:active:not(:disabled, [aria-disabled='true']) {
    transition-duration: var(--duration-press);
    background: var(--accent-deep);
  }

  .primary:is(:disabled, [aria-disabled='true']) {
    color: var(--bg);
    opacity: var(--opacity-disabled);
  }

  .danger {
    background: var(--status-down-tint);
    border-color: var(--status-down);
    color: var(--status-down);
  }
</style>
