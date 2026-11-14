<script lang="ts">
  import { press } from '$lib/motion/roles/press';

  interface Props {
    /** Whether the switch is on. */
    checked: boolean;
    /** The accessible name when no visible label names the switch; otherwise pass `labelledby`. */
    label?: string | undefined;
    /** The id of the visible text that names the switch. */
    labelledby?: string | undefined;
    id?: string | undefined;
    /** The switch cannot change now; it keeps focus and says so (`aria-disabled`). */
    disabled?: boolean;
    /** Runs with the new state after each flip by click, Space or Enter. */
    onchange?: (next: boolean) => void;
  }

  let { checked = $bindable(), label, labelledby, id, disabled = false, onchange }: Props = $props();

  function flip(): void {
    if (disabled) return;
    checked = !checked;
    onchange?.(checked);
  }
</script>

<!-- An on/off setting that takes effect at once (UR-7, D17): a switch, not a checkbox, so a screen reader says "on" or
     "off" rather than "checked". A button gives it Space and Enter for free. -->
<button
  type="button"
  role="switch"
  tabindex="0"
  {id}
  aria-checked={checked}
  aria-label={label}
  aria-labelledby={labelledby}
  aria-disabled={disabled || undefined}
  onclick={flip}
  {@attach press()}
></button>

<style>
  button {
    position: relative;
    flex: none;
    box-sizing: border-box;
    inline-size: calc(var(--space-8) + var(--space-1) / 2);
    block-size: var(--space-5);
    padding: 0;
    border: 0;
    border-radius: var(--radius-pill);
    background: var(--border-strong);
    cursor: pointer;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  /* The knob: a circle that slides to the end when on. */
  button::after {
    content: '';
    position: absolute;
    inset-block-start: calc(var(--space-1) - var(--border-thin));
    inset-inline-start: calc(var(--space-1) - var(--border-thin));
    inline-size: calc(var(--space-4) - var(--border-thin) * 2);
    block-size: calc(var(--space-4) - var(--border-thin) * 2);
    border-radius: var(--radius-pill);
    background: var(--surface-sunken);
    transition: translate var(--duration-base) var(--ease-enter);
  }

  button[aria-checked='true'] {
    background: var(--accent);
  }

  button[aria-checked='true']::after {
    translate: calc(var(--space-4) - var(--space-1) / 2) 0;
  }

  button[aria-disabled='true'] {
    opacity: var(--opacity-disabled);
    cursor: not-allowed;
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }

  @media (prefers-reduced-motion: reduce) {
    button,
    button::after {
      transition: none;
    }
  }
</style>
