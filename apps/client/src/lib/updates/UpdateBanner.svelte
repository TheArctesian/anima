<script lang="ts">
  import { crossFade } from '$lib/motion/roles/transitions';
  import { press } from '$lib/motion/roles/press';
  import { useUpdates } from './updates.svelte';

  const updates = useUpdates();
  let problem = $state<string | null>(null);
  const shown = $derived(updates.snapshot.state);

  async function restart(): Promise<void> {
    try {
      await updates.restart();
    } catch {
      problem = 'Anima could not restart. Quit it and open it again.';
    }
  }
</script>

<!-- Fixed over the page, so it never moves the content (UI-009). It restarts only when the operator presses the button (UPD-003). -->
{#if shown.kind === 'installed'}
  <div class="banner" role="status" transition:crossFade>
    <p>Anima {shown.version} is installed</p>
    <button type="button" tabindex="0" onclick={restart} {@attach press()}>Restart to update</button>
    {#if problem !== null}
      <p class="problem">{problem}</p>
    {/if}
  </div>
{/if}

<style>
  .banner {
    position: fixed;
    inset-block-end: var(--space-6);
    inset-inline-end: var(--space-6);
    z-index: var(--layer-toast);
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
    max-inline-size: calc(100vw - var(--space-12));
    padding: var(--space-3) var(--space-4);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    box-shadow: var(--elevation-2);
    color: var(--text);
    font-family: var(--font-ui);
  }

  p {
    margin: 0;
    font-size: var(--text-md);
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

  button:hover {
    background: var(--accent-tint);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .problem {
    flex-basis: 100%;
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }
</style>
