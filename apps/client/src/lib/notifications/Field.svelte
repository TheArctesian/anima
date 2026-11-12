<script lang="ts" module>
  /** What the control must carry so its label, hint and problem reach a screen reader. */
  export interface ControlAttributes {
    id: string;
    'aria-invalid': boolean;
    'aria-describedby': string;
  }
</script>

<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    label: string;
    /** A fixed hint under the control; never a value the operator typed. */
    hint?: string;
    /** Empty when the value is fine. A fixed sentence: it never repeats a secret (F-8). */
    problem: string;
    control: Snippet<[ControlAttributes]>;
  }

  const { label, hint, problem, control }: Props = $props();
  const id = $props.id();
  const attributes = $derived<ControlAttributes>({
    id,
    'aria-invalid': problem !== '',
    'aria-describedby': hint === undefined ? `${id}-problem` : `${id}-hint ${id}-problem`,
  });
</script>

<div class="field">
  <label for={id}>{label}</label>
  {@render control(attributes)}
  {#if hint !== undefined}<small id="{id}-hint">{hint}</small>{/if}
  <small id="{id}-problem" class="problem">{problem}</small>
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-inline-size: 0;
  }

  label {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  small {
    color: var(--text-secondary);
    font-size: var(--text-xs);
    line-height: var(--leading-normal);
  }

  /* The problem's line is kept when it is empty, so a refusal moves no field below it (MOT-007). */
  .problem {
    min-block-size: calc(var(--text-xs) * var(--leading-normal));
  }

  /* A problem is text with a bar, not colour alone (UI-003); --status-down text is under 4.5:1 (MAT-005). */
  .problem:not(:empty) {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
    color: var(--text);
  }

  .field :global(:is(input:not([type='checkbox'], [type='radio']), select, textarea)) {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: var(--space-2) var(--space-3);
    background-color: var(--surface-raised);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    color: var(--text);
    font: inherit;
  }

  .field :global(select) {
    padding-inline-end: var(--select-chevron-room);
  }

  .field :global(textarea) {
    resize: vertical;
  }

  .field :global(:disabled) {
    color: var(--text-secondary);
  }

  .field :global([aria-invalid='true']) {
    border-color: var(--status-down);
  }

  .field :global(:focus-visible) {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
