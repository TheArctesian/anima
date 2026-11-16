<script lang="ts">
  import FormButton from '../FormButton.svelte';

  interface Props {
    /** What the text is, above it. */
    label: string;
    /** What Copy puts on the clipboard. It may be a secret: it is never logged, put in an attribute or announced (LOG-007). */
    text: string;
    /** What the eye reads, when it differs from `text` (a grouped secret); `text` by default. */
    shown?: string;
  }

  const { label, text, shown = text }: Props = $props();
  const id = $props.id();
  let copy = $state<'idle' | 'copied' | 'failed'>('idle');

  async function copyText(): Promise<void> {
    try {
      await navigator.clipboard.writeText(text);
      copy = 'copied';
    } catch {
      // No clipboard, or the system refused it; the text stays selectable.
      copy = 'failed';
    }
  }
</script>

<div class="copy" role="group" aria-labelledby="{id}-label">
  <span class="label" id="{id}-label">{label}</span>
  <div class="row">
    <!-- Plain text, not a live region: a screen reader reads it only when the operator moves to it (LOG-007). -->
    <code class="text">{shown}</code>
    <FormButton tabindex={0} size="sm" onclick={copyText}>Copy</FormButton>
  </div>
  <!-- The live region holds only the outcome of Copy, never the text. -->
  <p class="outcome" role="status">{copy === 'copied' ? 'Copied.' : copy === 'failed' ? 'Anima.ing could not copy it. Select the text and copy it yourself.' : ''}</p>
</div>

<style>
  .copy {
    display: grid;
    gap: var(--space-1);
  }

  .label {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .row {
    display: flex;
    align-items: start;
    gap: var(--space-2);
  }

  .text {
    flex: 1 1 auto;
    min-inline-size: 0;
    padding: var(--space-2) var(--space-3);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
    word-break: break-all;
    user-select: all;
  }

  /* One line is reserved, so the outcome moves nothing below (MOT-007). */
  .outcome {
    margin: 0;
    min-block-size: calc(var(--text-sm) * var(--leading-normal));
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }
</style>
