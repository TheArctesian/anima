<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';

  interface Props {
    /** What failed, in plain words: "Devices did not load". */
    message: string;
    /** The next step's label (UI-030). */
    retryLabel?: string;
    onRetry: () => void;
    /** The technical cause, for a HelpTip; never shown as plain text here. */
    cause?: Snippet;
  }

  const { message, retryLabel = 'Try again', onRetry, cause }: Props = $props();
</script>

<!-- It replaces the content that failed to load and nothing else (UI-030). The text stays the text colour: --status-down text is under 4.5:1 (MAT-005). -->
<div class="error" role="alert">
  <span class="mark"><Icon name="nf-md-alert_circle_outline" size="lg" /></span>
  <p>{message}</p>
  <div class="next">
    <FormButton onclick={onRetry}>{retryLabel}</FormButton>
    {#if cause}{@render cause()}{/if}
  </div>
</div>

<style>
  .error {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-6) var(--space-4);
    text-align: center;
  }

  .mark {
    display: inline-flex;
    color: var(--status-down);
  }

  p {
    max-inline-size: 36ch;
    margin: 0;
    line-height: var(--leading-normal);
  }

  .next {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin-block-start: var(--space-2);
  }
</style>
