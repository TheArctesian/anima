<script lang="ts">
  import { onMount } from 'svelte';
  import { accessApi } from '$lib/access/api';
  import CodeBoxes from '$lib/access/code/CodeBoxes.svelte';
  import { tokenFor } from '$lib/access/listener/check';
  import { loadClientAt } from '$lib/access/navigation';
  import { storeHubAddress } from '$lib/desktop/address';
  import { checkHubListener } from '$lib/desktop/hub-listener/shell';
  import { readLocalHub } from '$lib/desktop/local-hub/shell';
  import { writeAccessToken } from '$lib/desktop/token';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { createRequestHub, type HubAddress, type RequestHub } from '$lib/hub/hub';
  import { secondsToNextCode } from '$lib/local-hub/account/account';
  import { createClock } from '$lib/nav/clock.svelte';
  import { connectLocalHub, signInFirst } from './connect';

  interface Props {
    /** The new account's name and password; `null` when the account existed before, so the sign-in page asks. */
    account: { name: string; password: string } | null;
    /** Signed in: the hub to make the invite with (FRN-002). */
    onsignedin: (hub: RequestHub) => void;
    clock?: { readonly current: Date };
  }

  const { account, onsignedin, clock = createClock(1000) }: Props = $props();
  let address = $state<HubAddress | null>(null);
  let problem = $state<string | null>(null);
  let busy = $state(false);
  let attempt = $state(0);

  onMount(() => {
    void connect();
  });

  async function connect(): Promise<void> {
    busy = true;
    problem = null;
    const connected = await connectLocalHub({
      read: readLocalHub,
      check: checkHubListener,
      saveAddress: (origin) => {
        storeHubAddress(localStorage, origin);
      },
    });
    busy = false;
    if (connected.ok) address = connected.address;
    else problem = connected.message;
  }

  async function signIn(code: string): Promise<void> {
    if (busy || address === null || account === null) return;
    const target = address;
    busy = true;
    problem = null;
    const hub = createRequestHub({ address: target, token: tokenFor(target, true) });
    const outcome = await signInFirst(accessApi(hub), (token) => writeAccessToken(target, token), { ...account, code });
    busy = false;
    if (outcome.ok) onsignedin(hub);
    else {
      problem = outcome.message;
      attempt += 1;
    }
  }
</script>

{#if address === null}
  <p role="status">{busy ? 'Checking the hub address on this computer.' : ''}</p>
  {#if !busy}<FormButton tone="primary" onclick={() => void connect()}>Try again</FormButton>{/if}
{:else if account === null}
  <p>This computer's hub already has an account. Sign in with it.</p>
  <FormButton tone="primary" onclick={() => loadClientAt('/sign-in')}>Continue to sign in</FormButton>
{:else}
  <p>Wait for the next code in your authenticator app: the code that made the account cannot sign in.</p>
  <p class="next" aria-live="off">The next code comes in {secondsToNextCode(clock.current)} s.</p>
  {#key attempt}
    <CodeBoxes disabled={busy} oncomplete={(code) => void signIn(code)} />
  {/key}
  <p role="status">{busy ? 'Signing in' : ''}</p>
{/if}
<p class="problem" role="alert">{problem ?? ''}</p>

<style>
  p {
    margin: 0;
    line-height: var(--leading-normal);
  }

  .next {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
  }

  .problem:not(:empty) {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }
</style>
