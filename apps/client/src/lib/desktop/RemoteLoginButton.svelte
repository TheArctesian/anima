<script lang="ts">
  import { press } from '$lib/motion/roles/press';
  import { openRemoteLoginSettings, remoteLoginSettingsAvailable, SystemSettingsError } from './systemSettings';

  interface Props {
    /** Shown only in the desktop app on a Mac (R-059). */
    available?: boolean;
  }

  const { available = remoteLoginSettingsAvailable() }: Props = $props();
  let problem = $state<string | null>(null);

  async function open(): Promise<void> {
    problem = null;
    try {
      await openRemoteLoginSettings();
    } catch (reason) {
      problem = reason instanceof SystemSettingsError ? reason.message : new SystemSettingsError('failed').message;
    }
  }
</script>

{#if available}
  <div class="remote-login">
    <p>Adding this Mac? SSH to it needs Remote Login turned on.</p>
    <button type="button" onclick={open} {@attach press()}>Open Remote Login settings</button>
    {#if problem !== null}
      <p class="problem" role="alert">{problem}</p>
    {/if}
  </div>
{/if}

<style>
  .remote-login {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
  }

  p {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .problem {
    color: var(--status-down);
  }

  button {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-4);
    background: var(--surface-raised);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-press) var(--ease-enter);
  }

  button:hover {
    background: var(--border);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
