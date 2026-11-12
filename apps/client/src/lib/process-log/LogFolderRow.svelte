<script lang="ts">
  import { LogFolderError, openLogFolder } from '$lib/desktop/process-log';
  import { press } from '$lib/motion/roles/press';
  import SettingRow from '$lib/settings/SettingRow.svelte';

  let problem = $state<string | null>(null);

  async function open(): Promise<void> {
    problem = null;
    try {
      await openLogFolder();
    } catch (reason) {
      problem =
        reason instanceof LogFolderError && reason.problem === 'unavailable'
          ? 'Opening the log folder is not available on this system.'
          : 'Anima could not open the log folder.';
    }
  }
</script>

<SettingRow label="Logs">
  {#snippet hint()}
    <p class="hint">The desktop app's process log, for a bug report.</p>
  {/snippet}
  <button type="button" tabindex="0" onclick={open} {@attach press()}>
    Open log folder
  </button>
  {#snippet message()}
    {#if problem !== null}<p role="alert">{problem}</p>{/if}
  {/snippet}
</SettingRow>

<style>
  button {
    box-sizing: border-box;
    block-size: var(--space-8);
    padding: 0 var(--space-4);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
    font-size: var(--text-md);
    white-space: nowrap;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover {
    background: var(--surface-hover);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
