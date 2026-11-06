<script lang="ts">
  import { press } from '$lib/motion/roles/press';
  import { openRemoteLoginSettings, remoteLoginSettingsAvailable, SystemSettingsError } from './systemSettings';

  interface Props {
    /** Shown only in the desktop app on a Mac (R-059). */
    available?: boolean;
    /** This Mac is being added and SSH did not answer on it (R-060): say so, and stand out. */
    sshOff?: boolean;
  }

  const { available = remoteLoginSettingsAvailable(), sshOff = false }: Props = $props();
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
  <div class="remote-login" class:off={sshOff}>
    {#if sshOff}
      <p class="why" role="status">SSH is off: it did not answer on this Mac. Turn on Remote Login to use it.</p>
    {:else}
      <p>Adding this Mac? SSH to it needs Remote Login turned on.</p>
    {/if}
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

  .off {
    padding: var(--space-3);
    background: var(--accent-tint);
    border: var(--border-thin) solid var(--accent);
    border-radius: var(--radius-md);
  }

  p {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .why {
    color: var(--text);
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
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover {
    background: var(--border);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
