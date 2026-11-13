<script lang="ts">
  import { tick } from 'svelte';
  import { press } from '$lib/motion/roles/press';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import { canRemove, describeCommandLine, installTarget, profileAdvice } from './command-line';
  import { CommandLineTool, desktopCommandLineShell, type CommandLineShell } from './command-line.svelte';

  const { shell = desktopCommandLineShell }: { shell?: CommandLineShell } = $props();

  // The row keeps the shell it was mounted with.
  // svelte-ignore state_referenced_locally
  const tool = new CommandLineTool(shell);
  let confirmButton = $state<HTMLButtonElement | null>(null);
  let installButton = $state<HTMLButtonElement | null>(null);

  $effect(() => {
    void tool.load();
  });

  const target = $derived(installTarget(tool.status));
  const advice = $derived(profileAdvice(tool.status));

  async function ask(): Promise<void> {
    tool.askInstall();
    await tick();
    confirmButton?.focus();
  }

  async function cancel(): Promise<void> {
    tool.cancel();
    await tick();
    installButton?.focus();
  }
</script>

<!-- The `anima` program the app carries, on the PATH through a link (macOS) or a launcher (Linux) (INS-004 to INS-006). -->
<SettingRow label="Command line tool">
  {#snippet hint()}
    <p class="hint state" aria-live="polite">
      {#if tool.confirming !== null}
        Install anima at <strong>{tool.confirming.path}</strong>?
      {:else}
        {describeCommandLine(tool.status)}
      {/if}
    </p>
    {#if advice !== null && tool.confirming === null}
      <p class="hint">
        {advice.folder} is not on your PATH in a new terminal. Add this line to your shell profile: <code>{advice.line}</code>
      </p>
    {/if}
  {/snippet}
  {#if tool.confirming !== null}
    <button type="button" bind:this={confirmButton} disabled={tool.busy} onclick={() => void tool.confirm()} {@attach press()}>
      Confirm
    </button>
    <button type="button" onclick={() => void cancel()} {@attach press()}>Cancel</button>
  {:else}
    <button type="button" bind:this={installButton} disabled={target === null || tool.busy} onclick={() => void ask()} {@attach press()}>
      Install
    </button>
    <button type="button" disabled={!canRemove(tool.status) || tool.busy} onclick={() => void tool.remove()} {@attach press()}>
      Remove
    </button>
  {/if}
  {#snippet message()}
    <p role="alert">{tool.problem ?? ''}</p>
  {/snippet}
</SettingRow>

<style>
  /* Two lines are reserved, so a long path does not move the rows below it (MOT-007). */
  .state {
    min-block-size: calc(2 * var(--text-sm) * var(--leading-tight));
    overflow-wrap: anywhere;
  }

  code {
    font-family: var(--font-mono);
    overflow-wrap: anywhere;
  }

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

  button:hover:enabled {
    background: var(--surface-hover);
  }

  button:disabled {
    color: var(--text-secondary);
    cursor: default;
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
