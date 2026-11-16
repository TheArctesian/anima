<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { resolve } from '$app/paths';
  import { loadClientAt } from '$lib/access/navigation';
  import { storeHubAddress } from '$lib/desktop/address';
  import { joinLink } from '$lib/desktop/link/shell';
  import { clearAccessToken } from '$lib/desktop/token';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { crossFade } from '$lib/motion/roles/transitions';
  import { joinHub } from './join';

  /** The invite as pasted: in this field only, never logged; dropped once it is sent (LOG-007). */
  let invite = $state('');
  let busy = $state(false);
  let problem = $state<string | null>(null);
  let fingerprint = $state<string | null>(null);
  let title: HTMLElement | undefined = $state();
  let done: HTMLElement | undefined = $state();

  onMount(() => {
    title?.focus();
  });

  async function join(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (busy) return;
    busy = true;
    problem = null;
    const sent = invite;
    invite = '';
    const outcome = await joinHub(sent, {
      join: joinLink,
      clearToken: clearAccessToken,
      saveAddress: (origin) => {
        storeHubAddress(localStorage, origin);
      },
    });
    busy = false;
    if (!outcome.ok) {
      problem = outcome.message;
      return;
    }
    fingerprint = outcome.fingerprint;
    await tick();
    done?.focus();
  }
</script>

<h1 tabindex="-1" bind:this={title}>Join a hub</h1>

{#if fingerprint === null}
  <form onsubmit={join} novalidate transition:crossFade>
    <label for="invite">Invite</label>
    <p id="invite-about" class="about">Make an invite on the hub computer, under Settings, Devices on the link. It works once, for 15 minutes.</p>
    <textarea
      id="invite"
      rows="4"
      autocomplete="off"
      autocapitalize="off"
      spellcheck="false"
      aria-describedby="invite-about invite-problem"
      aria-invalid={problem === null ? undefined : 'true'}
      bind:value={invite}
    ></textarea>
    <p id="invite-problem" class="problem" role="alert">{problem ?? ''}</p>
    <p class="busy" role="status">{busy ? 'Joining. This can take a minute.' : ''}</p>
    <div class="actions">
      <a href={resolve('/welcome')}>Back</a>
      <FormButton type="submit" tone="primary" {busy}>Join</FormButton>
    </div>
  </form>
{:else}
  <div class="joined" transition:crossFade>
    <h2 tabindex="-1" bind:this={done}>This device joined the hub</h2>
    <p>This device's key is</p>
    <p class="fingerprint">{fingerprint}</p>
    <p>Compare it with the device list on the hub. If the hub does not list it, the invite was not from your hub.</p>
    <FormButton tone="primary" onclick={() => loadClientAt('/sign-in')}>Continue to sign in</FormButton>
  </div>
{/if}

<style>
  form,
  .joined {
    display: grid;
    gap: var(--space-2);
  }

  label {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  p {
    margin: 0;
    line-height: var(--leading-normal);
  }

  .about,
  .busy {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  textarea {
    box-sizing: border-box;
    padding: var(--space-2) var(--space-3);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-base);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    word-break: break-all;
    resize: vertical;
  }

  textarea[aria-invalid='true'] {
    border-color: var(--status-down);
  }

  .problem,
  .busy {
    min-block-size: calc(var(--text-sm) * var(--leading-normal));
    font-size: var(--text-sm);
  }

  .problem:not(:empty) {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }

  .actions {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .actions a {
    color: var(--text-tertiary);
    font-size: var(--text-sm);
  }

  h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: var(--weight-medium);
  }

  .fingerprint {
    font-family: var(--font-mono);
    font-size: var(--text-lg);
  }

  :is(textarea, a, h2):focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }
</style>
