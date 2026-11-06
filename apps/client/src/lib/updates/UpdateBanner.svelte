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

<!--
  The status region is always in the page and empty until an update is installed, so screen readers announce what is put
  into it (UPD-004). It is fixed over the page, so it never moves the content (UI-009), and it restarts only when the
  operator presses the button (UPD-003). The banner fades with `crossFade`: it is a status that stays until restart, not a
  transient toast, so the toast role does not apply.
-->
<div class="live" role="status">
  {#if shown.kind === 'installed'}
    <div class="banner" transition:crossFade>
      <p>Anima {shown.version} is installed</p>
      <button type="button" tabindex="0" onclick={restart} {@attach press()}>Restart to update</button>
      {#if problem !== null}
        <p class="problem">{problem}</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .live {
    position: fixed;
    inset-block-end: var(--space-6);
    inset-inline-end: var(--space-6);
    z-index: var(--layer-toast);
    /* Never reaches the nav, however narrow the window (NAV-005). */
    max-inline-size: calc(100vw - var(--nav-inline-size) - var(--space-12));
  }

  .banner {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-3) var(--space-4);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    box-shadow: var(--elevation-2);
    color: var(--text);
    font-family: var(--font-ui);
    font-size: var(--text-md);
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
    background: var(--surface-raised);
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
