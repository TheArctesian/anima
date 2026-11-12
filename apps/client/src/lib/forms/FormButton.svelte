<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { HTMLButtonAttributes } from 'svelte/elements';
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import { press } from '$lib/motion/roles/press';
  import type { ActionState } from './feedback/feedback';

  interface Shape extends Omit<HTMLButtonAttributes, 'class' | 'children'> {
    /** Primary is the view's one main action (UI-029); `danger` is for the button that destroys something. */
    tone?: 'primary' | 'secondary' | 'ghost' | 'danger';
    /** Height: `control-sm` (dense rows), `control-md` (default) or `control-lg` (sign-in, empty states). */
    size?: 'sm' | 'md' | 'lg';
    /** A danger button is an outline; `fill` makes it solid, for the last step of a destructive flow. */
    fill?: boolean;
    /** The hub is working on this button's action: it keeps focus and its label, but a click does nothing. */
    busy?: boolean;
    /** For a button that starts hub work: its label gives way to `pendingLabel`, then to a check and `doneLabel`. */
    feedback?: ActionState;
    pendingLabel?: string;
    doneLabel?: string;
  }

  /** A labelled button may lead with an icon (UI-026); an icon-only button has no children, so it needs `label` (UI-015). */
  type Props = Shape & ({ children: Snippet; icon?: IconName; label?: undefined } | { children?: undefined; icon: IconName; label: string });

  const {
    tone = 'secondary',
    size = 'md',
    fill = false,
    busy = false,
    type = 'button',
    children,
    icon,
    label,
    feedback,
    pendingLabel = 'Saving',
    doneLabel = 'Saved',
    onclick,
    'aria-disabled': ariaDisabled,
    'aria-busy': ariaBusy,
    ...rest
  }: Props = $props();

  const isOn = (value: unknown) => value === true || value === 'true';
  const blocked = $derived(busy || isOn(ariaDisabled));
  /** What the button announces: its own state when it has one, else what the caller set (a caller's "false" stays). */
  const announcedBusy = $derived(busy || feedback === 'pending' ? true : ariaBusy);
  const announcedDisabled = $derived(busy ? true : ariaDisabled);

  /** A blocked button keeps focus, so it is not `disabled`; it just ignores the click and does not submit. */
  function click(event: MouseEvent & { currentTarget: HTMLButtonElement }): void {
    if (blocked) event.preventDefault();
    else onclick?.(event);
  }
</script>

<button
  class={[tone, size, fill && 'fill', label !== undefined && 'icon-only']}
  {type}
  title={label}
  aria-label={label}
  aria-busy={announcedBusy}
  aria-disabled={announcedDisabled}
  onclick={click}
  {...rest}
  {@attach press()}
>
  {#if icon !== undefined}<Icon name={icon} size="sm" />{/if}{#if feedback === undefined}
    {@render children?.()}
  {:else}
    <!-- All three labels share one cell, so the button keeps its width; each fades in place (MOT-007, MOT-008). -->
    <span class="labels">
      <span class="label" class:on={feedback === 'idle'} aria-hidden={feedback !== 'idle'}>{@render children?.()}</span>
      <span class="label waiting" class:on={feedback === 'pending'} aria-hidden={feedback !== 'pending'}>{pendingLabel}</span>
      <span class="label done" class:on={feedback === 'done'} aria-hidden={feedback !== 'done'}><Icon name="nf-md-check" size="sm" />{doneLabel}</span>
    </span>
  {/if}
</button>

<style>
  button {
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    block-size: var(--control-md);
    min-inline-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    background: none;
    border: var(--border-thin) solid transparent;
    border-radius: var(--radius-md);
    color: var(--text);
    font: inherit;
    font-weight: var(--weight-bold);
    white-space: nowrap;
    transition:
      background-color var(--duration-fast) var(--ease-enter),
      color var(--duration-fast) var(--ease-enter);
  }

  .sm {
    block-size: var(--control-sm);
    padding-inline: var(--space-2);
    font-size: var(--text-sm);
  }

  .lg {
    block-size: var(--control-lg);
  }

  /* Square: as wide as it is high, so a row of icon buttons shares the control height (UI-013). */
  .icon-only {
    aspect-ratio: 1;
    padding: 0;
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
    color: var(--text-secondary);
  }

  .done :global(.icon) {
    color: var(--status-up);
  }

  /* On a filled button the label keeps the button's own colour, for contrast (UI-004). */
  :is(.primary, .fill) :is(.waiting, .done :global(.icon)) {
    color: inherit;
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }

  button:is(:disabled, [aria-disabled='true']) {
    opacity: var(--opacity-disabled);
    cursor: not-allowed;
  }

  button[aria-busy='true'] {
    cursor: progress;
  }

  /* UI-004: accent fill, on-accent label; hover and pressed step along the accent ramp. */
  .primary {
    background: var(--accent);
    color: var(--on-accent);
  }

  .primary:hover:not(:disabled, [aria-disabled='true']) {
    background: var(--accent-hover);
  }

  .primary:active:not(:disabled, [aria-disabled='true']) {
    transition-duration: var(--duration-press);
    background: var(--accent-pressed);
  }

  /* The fill is already as light as a hover fill, so hover shows the strong border; pressed shows the accent tint. */
  .secondary {
    background: var(--surface-overlay);
  }

  .secondary:hover:not(:disabled, [aria-disabled='true']) {
    border-color: var(--border-strong);
  }

  .secondary:active:not(:disabled, [aria-disabled='true']) {
    transition-duration: var(--duration-press);
    background: var(--surface-selected);
  }

  .ghost {
    color: var(--text-secondary);
    font-weight: var(--weight-regular);
  }

  .ghost:hover:not(:disabled, [aria-disabled='true']) {
    background: var(--surface-hover);
    color: var(--text);
  }

  .ghost:active:not(:disabled, [aria-disabled='true']) {
    transition-duration: var(--duration-press);
    background: var(--surface-selected);
    color: var(--text);
  }

  .danger {
    border-color: var(--danger);
    color: var(--status-down-text);
  }

  .danger:is(:hover, :active):not(:disabled, [aria-disabled='true']) {
    background: var(--status-down-tint);
  }

  .danger.fill {
    background: var(--danger);
    color: var(--on-danger);
  }

  .danger.fill:hover:not(:disabled, [aria-disabled='true']) {
    background: var(--danger);
    filter: brightness(1.1);
  }

  .danger.fill:active:not(:disabled, [aria-disabled='true']) {
    background: var(--danger);
    filter: brightness(0.9);
  }
</style>
