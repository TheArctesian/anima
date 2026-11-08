<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';

  interface Props {
    /** The whole key. Shown here and nowhere else: never in a title, a live region, a URL, storage or the console (INT-003, LOG-007). */
    secret: string;
  }

  const { secret }: Props = $props();
  const fieldId = $props.id();
  let copy = $state<'idle' | 'copied' | 'failed'>('idle');
  let field: HTMLInputElement | undefined = $state();

  const COPIED = 'Copied.';
  const COPY_FAILED = 'Anima could not copy the key. Select it in the field and copy it yourself.';

  async function copyKey(): Promise<void> {
    try {
      await navigator.clipboard.writeText(secret);
      copy = 'copied';
    } catch {
      // No clipboard, or the system refused it; the reason is not shown, and the field stays selectable.
      copy = 'failed';
    }
  }

  // The field takes focus when it appears, with the key selected, so the system's own copy works at once.
  $effect(() => {
    field?.focus();
  });
</script>

<div class="secret">
  <label for={fieldId}>API key</label>
  <div class="row">
    <input
      bind:this={field}
      id={fieldId}
      type="text"
      readonly
      value={secret}
      autocomplete="off"
      autocapitalize="off"
      spellcheck="false"
      onfocus={(event) => {
        event.currentTarget.select();
      }}
    />
    <FormButton tabindex={0} onclick={copyKey}>Copy</FormButton>
  </div>
  <!-- The live region holds only the outcome of Copy, never the key. -->
  <p class="outcome" class:failed={copy === 'failed'} role="status">
    {copy === 'copied' ? COPIED : copy === 'failed' ? COPY_FAILED : ''}
  </p>
</div>

<style>
  .secret {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  label {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .row {
    display: flex;
    gap: var(--space-2);
  }

  input {
    box-sizing: border-box;
    flex: 1 1 auto;
    min-inline-size: 0;
    min-block-size: var(--target-min-desktop);
    padding: var(--space-2) var(--space-3);
    background-color: var(--surface);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }

  input:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  /* One line is reserved, so the outcome's arrival does not move the dialog's buttons (MOT-007). */
  .outcome {
    min-block-size: calc(var(--text-sm) * var(--leading-normal));
    margin: 0;
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  /* --status-down text is 3.05:1 (MAT-005 needs 4.5:1): the text stays --text and a bar marks the failure. */
  .failed {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }
</style>
