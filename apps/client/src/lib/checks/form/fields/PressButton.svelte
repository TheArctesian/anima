<script lang="ts">
  import { press } from '$lib/motion/roles/press';

  interface Props {
    label: string;
    kind?: 'primary' | 'quiet';
    submit?: boolean;
    /** The ID of the form a submit button outside that form submits. */
    form?: string;
    disabled?: boolean;
    onclick?: () => void;
  }

  const { label, kind = 'quiet', submit = false, form, disabled = false, onclick }: Props = $props();
</script>

<button
  class={kind}
  type={submit ? 'submit' : 'button'}
  {form}
  {disabled}
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
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover:not(:disabled, .primary) {
    background: var(--surface-hover);
  }

  button:disabled {
    color: var(--text-muted);
  }

  /* UI-004: accent with a --bg label, accent-deep while pressed, as WakeButton. */
  .primary {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--bg);
  }

  .primary:active:not(:disabled) {
    background: var(--accent-deep);
  }

  .primary:disabled {
    color: var(--bg);
    opacity: var(--opacity-disabled);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
