<script lang="ts">
  import { press } from '$lib/motion/roles/press';

  interface Props {
    label: string;
    kind?: 'primary' | 'quiet';
    submit?: boolean;
    /** The ID of the form a submit button outside that form submits. */
    form?: string;
    disabled?: boolean;
    /** Working: the button stays focusable and says so with `aria-disabled`; the owner ignores the press. */
    busy?: boolean;
    onclick?: () => void;
  }

  const { label, kind = 'quiet', submit = false, form, disabled = false, busy = false, onclick }: Props = $props();
</script>

<button
  class={kind}
  type={submit ? 'submit' : 'button'}
  {form}
  {disabled}
  aria-disabled={busy ? 'true' : undefined}
  {onclick}
  {@attach press()}
>
  {label}
</button>

<style>
  button {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-4);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-overlay);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover:not(:disabled, [aria-disabled='true'], .primary) {
    background: var(--surface-hover);
  }

  button:disabled,
  button[aria-disabled='true'] {
    color: var(--text-secondary);
  }

  /* UI-004: accent with a --surface-base label, accent-deep while pressed, as WakeButton. */
  .primary {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--surface-base);
  }

  .primary:active:not(:disabled, [aria-disabled='true']) {
    background: var(--accent-pressed);
  }

  .primary:is(:disabled, [aria-disabled='true']) {
    color: var(--surface-base);
    opacity: var(--opacity-disabled);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
