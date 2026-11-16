<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { resolve } from '$app/paths';
  import { loadClientAt } from '$lib/access/navigation';
  import { storeHubAddress } from '$lib/desktop/address';
  import { joinLink, type Joined } from '$lib/desktop/link/shell';
  import { clearAccessToken } from '$lib/desktop/token';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { crossFade } from '$lib/motion/roles/transitions';
  import { finishJoin, joinHub, type Finished } from './join';

  /** The invite as pasted: in this field only, never logged; dropped once the hub may have used it (LOG-007). */
  let invite = $state('');
  let busy = $state(false);
  let problem = $state<string | null>(null);
  /** The joined device: its key and link port, which are not secrets. */
  let device = $state<Joined | null>(null);
  let finished = $state<Finished | null>(null);
  let title: HTMLElement | undefined = $state();
  let done: HTMLElement | undefined = $state();

  const after = {
    clearToken: clearAccessToken,
    saveAddress: (origin: string) => {
      storeHubAddress(localStorage, origin);
    },
  };

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
    const outcome = await joinHub(sent, { join: joinLink, ...after });
    busy = false;
    if (!outcome.joined) {
      problem = outcome.message;
      // A refusal before the hub saw the invite leaves it usable: the field gets it back.
      if (outcome.inviteKept) invite = sent;
      return;
    }
    ({ device, finished } = outcome);
    await tick();
    done?.focus();
  }

  /** The invite is spent: Retry repeats only the steps after the join. */
  async function retry(): Promise<void> {
    if (busy || device === null) return;
    busy = true;
    finished = await finishJoin(device, after);
    busy = false;
  }
</script>

<h1 tabindex="-1" bind:this={title}>Join a hub</h1>

<!-- The form and the joined view share one cell, so they cross-fade in place, as the create steps do (MOT-008). -->
<div class="stack">
{#if device === null}
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
    <p class="fingerprint">{device.fingerprint}</p>
    <p>Compare it with the device list on the hub. If the hub does not list it, the invite was not from your hub.</p>
    <p class="problem" role="alert">{finished?.ok === false ? finished.message : ''}</p>
    {#if finished?.ok === true}
      <FormButton tone="primary" onclick={() => loadClientAt('/sign-in')}>Continue to sign in</FormButton>
    {:else}
      <FormButton tone="primary" {busy} onclick={() => void retry()}>Retry</FormButton>
    {/if}
  </div>
{/if}
</div>

<style>
  .stack {
    display: grid;
  }

  form,
  .joined {
    grid-area: 1 / 1;
    display: grid;
    gap: var(--space-2);
    align-content: start;
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
    display: inline-flex;
    align-items: center;
    min-block-size: var(--target-min-desktop);
    min-inline-size: var(--target-min-desktop);
    color: var(--text-secondary);
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
