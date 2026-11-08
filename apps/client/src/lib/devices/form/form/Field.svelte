<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { FieldError } from '$lib/hub/hub';
  import { errorFor, errorId, inputId } from './errors';

  /** What the control must carry so the label and the error reach a screen reader. */
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
    'aria-describedby': error === undefined ? undefined : errorId(field),
  });
</script>

<div class="field">
  <label for={inputId(field)}>{label}</label>
  {@render control(attributes)}
  {#if hint !== undefined}
    <small>{hint}</small>
  {/if}
  <small id={errorId(field)} class="error">{error?.message ?? ''}</small>
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  label {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  small {
    color: var(--text-muted);
    font-size: var(--text-xs);
  }

  .error {
    color: var(--status-down);
  }

  .error:empty {
    display: none;
  }

  .field :global(input:not([type='checkbox'])),
  .field :global(select),
  .field :global(textarea) {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: var(--space-2) var(--space-3);
    background-color: var(--surface);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font: inherit;
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
    font-family: var(--font-mono);
    resize: vertical;
  }
</style>
