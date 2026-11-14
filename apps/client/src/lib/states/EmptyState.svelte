<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** What is empty, in a few words: "No devices yet". */
    title: string;
    /** One line: what the operator can do about it. */
    line: string;
    /** The one action (UI-030), for example an "Add device" button. */
    action?: Snippet;
  }

  const { title, line, action }: Props = $props();
</script>

<!-- Sized to its message (UI-027): no fixed height, no padding to fill a card. -->
<div class="empty">
  <h3>{title}</h3>
  <p>{line}</p>
  {#if action}<div class="action">{@render action()}</div>{/if}
</div>

<style>
  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-6) var(--space-4);
    text-align: center;
  }

  h3 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: var(--weight-medium);
    line-height: var(--leading-tight);
  }

  p {
    max-inline-size: 36ch;
    margin: 0;
    color: var(--text-secondary);
    line-height: var(--leading-normal);
  }

  .action {
    margin-block-start: var(--space-2);
  }
</style>
