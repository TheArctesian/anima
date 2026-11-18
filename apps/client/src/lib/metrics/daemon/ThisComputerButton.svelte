<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import type { ThisComputerInstall } from './this-computer.svelte';

  interface Props {
    install: ThisComputerInstall;
    /** Runs after the daemon is installed, so the panel reads the daemon state again. */
    onInstalled: () => void;
  }

  const { install, onInstalled }: Props = $props();

  async function start(): Promise<void> {
    if (await install.install()) onInstalled();
  }
</script>

{#if install.installed}
  <p role="status">Daemon installed and running.</p>
{/if}
{#if install.error !== null}
  <p role="alert">{install.error}</p>
{/if}
<FormButton onclick={() => void start()} aria-disabled={install.busy} aria-busy={install.busy}>
  {install.busy ? 'Installing…' : 'Install on this computer'}
</FormButton>
