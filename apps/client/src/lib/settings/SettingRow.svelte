<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** The row's name. A string is drawn as a `<label>` for the control with the id `for`, or as text without one. */
    label: string | Snippet;
    /** The id of the control the label names. */
    for?: string;
    /** A short muted line under the name; give its element the class `hint`. */
    hint?: Snippet;
    /** The control, at the end of the line. */
    children: Snippet;
    /** The row's message line under it; give its element `role="alert"`. It keeps one line while empty (MOT-007). */
    message?: Snippet;
  }

  const { label, for: control, hint, children, message }: Props = $props();
</script>

<!-- One setting: its name (and hint) at the start of the line, its control at the end; stacked only in a narrow card. -->
<div class="row">
  <div class="line">
    <div class="text">
      {#if typeof label !== 'string'}
        {@render label()}
      {:else if control === undefined}
        <span class="name">{label}</span>
      {:else}
        <label class="name" for={control}>{label}</label>
      {/if}
      {@render hint?.()}
    </div>
    <div class="control">{@render children()}</div>
  </div>
  {@render message?.()}
</div>

<style>
  .row {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-3) var(--space-4);
    border-block-start: var(--border-thin) solid var(--border);
  }

  .line {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: var(--space-2) var(--space-4);
    align-items: center;
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-inline-size: 0;
  }

  .control {
    display: flex;
    align-items: center;
    justify-content: end;
    gap: var(--space-2);
  }

  /* The controls of every row share one look and one height, so the rows line up (SET-001). */
  .control :global(:is(select, input)) {
    box-sizing: border-box;
    block-size: var(--space-8);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background-color: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: var(--text-md);
  }

  .control :global(select) {
    padding: 0 var(--select-chevron-room) 0 var(--space-2);
  }

  .control :global(input) {
    inline-size: calc(var(--space-8) * 2.5);
    padding: 0 var(--space-2);
    text-align: end;
  }

  .control :global(:is(select, input):focus-visible) {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .control :global(input[aria-invalid='true']) {
    border-color: var(--status-down);
  }

  /* The name's line is reserved from tokens, so a font load cannot move the row (MOT-007). */
  .name {
    min-block-size: calc(var(--text-md) * var(--leading-tight));
    font-size: var(--text-md);
    line-height: var(--leading-tight);
  }

  .text > :global(.hint) {
    margin: 0;
    min-block-size: calc(var(--text-sm) * var(--leading-tight));
    color: var(--text-muted);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  /* One line is kept for the message, so its arrival moves nothing below (MOT-007, MV-005).
     --status-down text is 3.05:1 (MAT-005 needs 4.5:1): the text stays --text and a bar marks the error. */
  .row > :global([role='alert']) {
    margin: 0;
    min-block-size: calc(var(--text-sm) * var(--leading-tight));
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid transparent;
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  .row > :global([role='alert']:not(:empty)) {
    border-inline-start-color: var(--status-down);
  }

  /* A narrow card stacks the control under its name. */
  @container (max-inline-size: 28rem) {
    .line {
      grid-template-columns: minmax(0, 1fr);
    }

    .control {
      justify-content: start;
    }
  }
</style>
