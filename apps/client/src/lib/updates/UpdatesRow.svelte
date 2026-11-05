<script lang="ts">
  import { press } from '$lib/motion/roles/press';
  import { describeUpdate, canCheck } from './status';
  import { useUpdates } from './updates.svelte';

  const updates = useUpdates();
  const snapshot = $derived(updates.snapshot);

  $effect(() => {
    void updates.loadVersion();
  });

  const time = (at: Date): string => at.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
</script>

<section aria-label="Updates">
  <!-- Both lines are always rendered with their height reserved, so the button does not move when they fill (MOT-007). -->
  <p class="version">
    {#if updates.current !== null}Anima <strong>{updates.current}</strong>{/if}
  </p>
  <p class="state">{describeUpdate(snapshot, time)}</p>
  <button type="button" tabindex="0" disabled={!canCheck(snapshot)} onclick={() => void updates.checkNow()} {@attach press()}>
    Check for updates
  </button>
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    align-items: start;
    gap: var(--space-2);
    margin-block-start: var(--space-6);
  }

  p {
    margin: 0;
    min-block-size: calc(var(--text-md) * var(--leading-tight));
    font-size: var(--text-md);
    line-height: var(--leading-tight);
  }

  .state {
    color: var(--text-muted);
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
    transition: background-color var(--duration-press) var(--ease-enter);
  }

  button:hover:enabled {
    background: var(--surface-raised);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
