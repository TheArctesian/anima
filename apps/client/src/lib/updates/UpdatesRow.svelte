<script lang="ts">
  import { press } from '$lib/motion/roles/press';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import { describeUpdate, canCheck } from './status';
  import { useUpdates } from './updates.svelte';

  const updates = useUpdates();
  const snapshot = $derived(updates.snapshot);

  $effect(() => {
    void updates.loadVersion();
  });

  const time = (at: Date): string => at.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
</script>

<SettingRow>
  {#snippet label()}
    <!-- Both lines are always rendered with their height reserved, so nothing moves when they fill (MOT-007). -->
    <p class="version">
      {#if updates.current !== null}Anima <strong>{updates.current}</strong>{/if}
    </p>
  {/snippet}
  {#snippet hint()}
    <p class="hint state" aria-live="polite">{describeUpdate(snapshot, time)}</p>
  {/snippet}
  <button type="button" tabindex="0" disabled={!canCheck(snapshot)} onclick={() => void updates.checkNow()} {@attach press()}>
    Check for updates
  </button>
</SettingRow>

<style>
  .version {
    margin: 0;
    min-block-size: calc(var(--text-md) * var(--leading-tight));
    font-size: var(--text-md);
    line-height: var(--leading-tight);
  }

  /* Two lines are reserved, so a wrapped sentence does not move the rows below it (MOT-007). */
  .state {
    min-block-size: calc(2 * var(--text-sm) * var(--leading-tight));
  }

  button {
    box-sizing: border-box;
    block-size: var(--space-8);
    padding: 0 var(--space-4);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-size: var(--text-md);
    white-space: nowrap;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover:enabled {
    background: var(--surface-hover);
  }

  button:disabled {
    color: var(--text-muted);
    cursor: default;
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
