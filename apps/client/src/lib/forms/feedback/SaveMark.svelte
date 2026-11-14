<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';

  interface Props {
    /** Where the setting's save stands; nothing shows before its first change. */
    mark: 'saving' | 'saved' | 'faded' | 'refused' | undefined;
  }

  const { mark }: Props = $props();
</script>

<!-- Beside a setting's label (UR-7, D17): "Saving…" turns into "Saved", which fades; "Not saved" stays until the value
     saves. The region is always there, so a screen reader hears each change. -->
<span class="mark {mark ?? ''}" role="status">
  {#if mark === 'saving'}
    Saving…
  {:else if mark === 'saved' || mark === 'faded'}
    <Icon name="carbon:checkmark" size="sm" />Saved
  {:else if mark === 'refused'}
    <Icon name="carbon:warning-alt" size="sm" />Not saved
  {/if}
</span>

<style>
  .mark {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    margin-inline-start: var(--space-2);
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    font-weight: var(--weight-regular);
    line-height: var(--leading-tight);
    vertical-align: middle;
    white-space: nowrap;
    transition: opacity var(--duration-base) var(--ease-exit);
  }

  .saved,
  .faded {
    color: var(--status-up-text);
  }

  .faded {
    opacity: 0;
  }

  .refused {
    color: var(--status-down-text);
  }

  @media (prefers-reduced-motion: reduce) {
    .mark {
      transition: none;
    }
  }
</style>
