<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { FieldError } from '$lib/hub/hub';
  import { describedBy, errorFor, errorId, hintId, inputId } from './errors';

  /** What the control must carry so the label, the hint and the error reach a screen reader. */
  export interface ControlAttributes {
    id: string;
    'aria-invalid': boolean;
    'aria-describedby': string | undefined;
  }

  interface Props {
    label: string;
    /** The field path, for example `ssh.port`. */
    field: string;
    errors: readonly FieldError[];
    hint?: string;
    control: Snippet<[ControlAttributes]>;
  }

  const { label, field, errors, hint, control }: Props = $props();
  const error = $derived(errorFor(errors, field));
  const attributes = $derived<ControlAttributes>({
    id: inputId(field),
    'aria-invalid': error !== undefined,
    // The hint, then the error: both are read after the label.
    'aria-describedby': describedBy(field, hint !== undefined, error !== undefined),
  });
</script>

<div class="field">
  <label for={inputId(field)}>{label}</label>
  {@render control(attributes)}
  {#if hint !== undefined}
    <small id={hintId(field)}>{hint}</small>
  {/if}
  <small id={errorId(field)} class="error">{error?.message ?? ''}</small>
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-inline-size: 0;
  }

  /* The label in the text colour, the hint below the control in the muted one, so they read apart. */
  label {
    color: var(--text);
    font-size: var(--text-sm);
  }

  small {
    color: var(--text-muted);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  .error {
    color: var(--status-down);
  }

  .error:empty {
    display: none;
  }

  /* Every control the same height and type size, whatever its kind (an input with a list, a select). */
  .field :global(input:not([type='checkbox'], [type='radio'])),
  .field :global(select),
  .field :global(textarea) {
    box-sizing: border-box;
    inline-size: 100%;
    min-block-size: var(--space-8);
    margin: 0;
    padding: 0 var(--space-3);
    background-color: var(--surface);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font: inherit;
    font-size: var(--text-md);
    line-height: var(--leading-tight);
  }

  .field :global(select) {
    padding-inline-end: var(--select-chevron-room);
  }

  .field :global([aria-invalid='true']) {
    border-color: var(--status-down);
  }

  .field :global(:focus-visible) {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .field :global(textarea) {
    padding-block: var(--space-2);
    font-family: var(--font-mono);
    line-height: var(--leading-normal);
    resize: vertical;
  }
</style>
