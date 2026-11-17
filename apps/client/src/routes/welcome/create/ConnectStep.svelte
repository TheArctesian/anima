<script lang="ts">
  import { onMount } from 'svelte';
  import { accessApi } from '$lib/access/api';
  import CodeBoxes from '$lib/access/code/CodeBoxes.svelte';
  import { tokenFor } from '$lib/access/listener/check';
  import { storeHubAddress } from '$lib/desktop/address';
  import { checkHubListener } from '$lib/desktop/hub-listener/shell';
  import { readLocalHub } from '$lib/desktop/local-hub/shell';
  import { writeAccessToken } from '$lib/desktop/token';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { createRequestHub, type HubAddress, type RequestHub } from '$lib/hub/hub';
  import { secondsToNextCode } from '$lib/local-hub/account/account';
  import { createClock } from '$lib/nav/clock.svelte';
  import { connectLocalHub, signInFirst } from './connect';
  import ExistingAccount from './ExistingAccount.svelte';

  interface Props {
    /** The new account's name and password; `null` when the account existed before, so this step asks for them. */
    account: { name: string; password: string } | null;
    /** Signed in: the hub to make the invite with (FRN-002). */
    onsignedin: (hub: RequestHub) => void;
    clock?: { readonly current: Date };
    /** Waits between reads of a hub that is not ready yet; tests pass their own (CQ-021). */
    wait?: (ms: number) => Promise<void>;
  }

  const { account, onsignedin, clock = createClock(1000), wait = (ms) => new Promise((resolve) => setTimeout(resolve, ms)) }: Props = $props();
  /** The account made on the step before keeps its sentence about the next code. */
  // svelte-ignore state_referenced_locally
  const made = account !== null;
  /** The name and password to sign in with: secrets until signed in (LOG-007). */
  // svelte-ignore state_referenced_locally
  let signer = $state(account);
  let address = $state<HubAddress | null>(null);
  let problem = $state<string | null>(null);
  let busy = $state(false);
  let attempt = $state(0);
  let gone = false;

  onMount(() => {
    void connect();
    return () => {
      gone = true;
    };
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
      // A step that has gone stops reading: its wait never ends.
      wait: (ms) => (gone ? new Promise<void>(() => undefined) : wait(ms)),
    });
    busy = false;
    if (connected.ok) address = connected.address;
    else problem = connected.message;
  }

  async function signIn(code: string): Promise<void> {
    if (busy || address === null || signer === null) return;
    const target = address;
    busy = true;
    problem = null;
    const hub = createRequestHub({ address: target, token: tokenFor(target, true) });
    const outcome = await signInFirst(accessApi(hub), (token) => writeAccessToken(target, token), { ...signer, code });
    busy = false;
    if (outcome.ok) {
      signer = null;
      onsignedin(hub);
    } else {
      problem = outcome.message;
      attempt += 1;
    }
  }
</script>

{#if address === null}
  <p role="status">{busy ? 'Checking the hub address on this computer.' : ''}</p>
  {#if !busy}<FormButton tone="primary" onclick={() => void connect()}>Retry</FormButton>{/if}
{:else if signer === null}
  <ExistingAccount onnext={(chosen) => (signer = chosen)} />
{:else}
  <p>{made ? 'Wait for the next code: the one that made the account cannot sign in.' : 'Enter your authenticator code.'}</p>
  <p class="next" aria-live="off">Next code in {secondsToNextCode(clock.current)} s.</p>
  <!-- The step heading has focus when the step shows; the boxes take it only after a refused code. -->
  <div class="code">
    {#key attempt}
      <CodeBoxes disabled={busy} autofocus={attempt > 0 || !made} oncomplete={(code) => void signIn(code)} />
    {/key}
  </div>
  <p class="status" role="status">{busy ? 'Signing in' : ''}</p>
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

  /* The boxes' row and the status and problem lines keep their height, so nothing moves while signing in (MOT-007). */
  .code {
    min-block-size: var(--control-lg);
  }

  .status,
  .problem {
    min-block-size: calc(var(--text-md) * var(--leading-normal));
  }

  .problem:not(:empty) {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }
</style>
