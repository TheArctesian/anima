<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { asset } from '$app/paths';
  import { accessApi } from '$lib/access/api';
  import { goToDevices, loadClientAt, reloadClient } from '$lib/access/navigation';
  import { storeHubAddress } from '$lib/desktop/address';
  import { readAccessToken, writeAccessToken } from '$lib/desktop/token';
  import { useHub } from '$lib/hub/context';
  import { createHub, parseHubAddress, type Hub, type HubAddress } from '$lib/hub/hub';
  import { press } from '$lib/motion/roles/press';
  import { createClock } from '$lib/nav/clock.svelte';
  import { DATA_AGE_REFRESH_MS } from '$lib/nav/nav';
  import { capabilities } from '$lib/platform/platform';
  import CodeStep from './code/CodeStep.svelte';
  import { afterEdit, checkLogin, firstInvalid, type FieldMessages, type LoginField } from './form';
  import HubLine from './HubLine.svelte';
  import Notice from './Notice.svelte';
  import { hubHost, hubUnreachable } from './reachability';
  import { saveHubAddress, signIn } from './sign-in';
  import TextField from './TextField.svelte';

  const hub = useHub();
  const clock = createClock(DATA_AGE_REFRESH_MS);
  // Read once: the platform never changes in a page, so the hub line below never comes or goes (CLI-002).
  const desktop = capabilities().keyStore;

  let username = $state('');
  let password = $state('');
  // Step 1 asks for the user name and password and sends nothing; step 2 asks for the code and sends all three (NAV-009).
  let step = $state<'credentials' | 'code'>('credentials');
  let fields = $state<FieldMessages>({});
  let message = $state<string | null>(null);
  let busy = $state(false);
  let inputs: Partial<Record<'username' | 'password', HTMLInputElement>> = $state({});

  let editingAddress = $state(false);
  let addressText = $state(hub.address.origin);
  let addressProblem = $state<string | undefined>();

  const unreachable = $derived(hubUnreachable(hub.status.current, hub.address.origin, clock.current));

  onMount(() => {
    inputs.username?.focus();
  });

  /** The address typed into the open hub field: `undefined` when the field is closed or unchanged, `null` (with the problem shown) when it is refused. */
  function typedAddress(): HubAddress | undefined | null {
    if (!editingAddress) return undefined;
    const parsed = parseHubAddress(addressText);
    if (!parsed.ok) {
      addressProblem = parsed.error.message;
      document.getElementById('hub-address')?.focus();
      return null;
    }
    return parsed.value.origin === hub.address.origin ? undefined : parsed.value;
  }

  /** The hub to sign in to: the current one, or one for the address typed into the open hub field. */
  function chooseTarget(moved: HubAddress | undefined): { target: Hub; moved: HubAddress | undefined } {
    if (moved === undefined) return { target: hub, moved };
    return { target: createHub({ address: moved, token: readAccessToken }), moved };
  }

  /** Step 1's Continue: check the two fields here and move on; nothing is sent. */
  function continueToCode(): void {
    message = null;
    // A bad hub address is told now, while the form is still open to fix it (CLI-002).
    if (typedAddress() === null) return;
    fields = checkLogin({ username, password, totpCode: '' });
    const first = firstInvalid(fields);
    if (first === undefined) {
      step = 'code';
      return;
    }
    inputs[first === 'username' ? 'username' : 'password']?.focus();
  }

  async function backToCredentials(): Promise<void> {
    step = 'credentials';
    await tick();
    inputs.password?.focus();
  }

  function formSubmitted(event: SubmitEvent): void {
    event.preventDefault();
    if (step === 'credentials') continueToCode();
  }

  async function submit(totpCode: string): Promise<void> {
    // One attempt at a time: a second browser sign-in would leave a token on the hub that no cookie holds.
    if (busy) return;
    const typed = typedAddress();
    if (typed === null) return;
    const { target, moved } = chooseTarget(typed);
    const form = { username, password, totpCode };
    // The secrets leave the page state as soon as they are read; only the user name stays (LOG-007).
    password = '';
    busy = true;
    const outcome = await signIn({ api: accessApi(target), address: target.address, desktop, writeToken: writeAccessToken }, form);
    if (target !== hub) closeCandidate(target);
    if (outcome.signedIn && (!desktop || moved !== undefined)) {
      // A new page, so the socket opens with the cookie the hub just set (E-4), or with the address just chosen (CLI-002);
      // the button stays off while it loads.
      if (moved !== undefined) storeHubAddress(localStorage, moved.origin);
      loadClientAt('/devices');
      return;
    }
    busy = false;
    if (outcome.signedIn) {
      goToDevices();
      return;
    }
    // Every refusal ends on step 1 with the user name kept and the password and code gone (NAV-009, SEC-002).
    message = outcome.message;
    fields = outcome.fields;
    step = 'credentials';
    await tick();
    inputs[firstInvalid(fields) === 'username' ? 'username' : 'password']?.focus();
  }

  function closeCandidate(candidate: Hub): void {
    candidate.close().catch((reason: unknown) => {
      // Fixed text and the error name only: the message is decided by the transport (LOG-007).
      console.warn('Hub connection attempt failed while closing:', reason instanceof Error ? reason.name : 'unknown');
    });
  }

  function edited(field: LoginField, event: Event & { currentTarget: HTMLInputElement }): void {
    fields = afterEdit(fields, field, event.currentTarget.value);
  }

  function saveAddress(): void {
    addressProblem = saveHubAddress(localStorage, addressText, reloadClient) ?? undefined;
  }

  function cancelAddress(): void {
    editingAddress = false;
    addressText = hub.address.origin;
    addressProblem = undefined;
  }
</script>

<div class="screen">
  <header>
    <img src={asset('/logo.png')} alt="" width="32" height="32" />
    <h1>Sign in to Anima</h1>
  </header>

  <form class="card" onsubmit={formSubmitted} novalidate>
    {#if unreachable !== null}
      <div role="status">
        <Notice icon="nf-md-lan_disconnect">
          <p><strong>{unreachable.title}</strong></p>
          <p>{unreachable.advice}</p>
          <p class="detail"><code>{unreachable.origin}</code> · {unreachable.age}</p>
        </Notice>
      </div>
    {/if}

    {#if step === 'credentials'}
      <div class="group">
        <TextField id="username" name="username" label="User name" autocomplete="username" bind:value={username} bind:input={inputs.username}
          error={fields.username} oninput={(event) => { edited('username', event); }} />
        <TextField id="password" name="password" label="Password" type="password" autocomplete="current-password" bind:value={password}
          bind:input={inputs.password} error={fields.password} oninput={(event) => { edited('password', event); }} />
      </div>
    {:else}
      <CodeStep {busy} oncomplete={(code) => void submit(code)} onback={() => void backToCredentials()} onskip={() => void submit('')} />
    {/if}

    {#if message !== null}
      <Notice icon="nf-md-alert_circle_outline"><p role="alert">{message}</p></Notice>
    {/if}

    {#if step === 'credentials'}
      <!-- tabindex="0": WebKit skips plain buttons when Tab is pressed unless they are focusable, as for the nav links (NAV-005, WP-11 H-6). -->
      <button type="submit" tabindex="0" {@attach press()}>Continue</button>
    {/if}
  </form>

  {#if desktop}
    <HubLine host={hubHost(hub.address.origin)} bind:editing={editingAddress} bind:text={addressText} problem={addressProblem}
      onsave={saveAddress} oncancel={cancelAddress} />
  {/if}
</div>

<style>
  .screen {
    box-sizing: border-box;
    display: grid;
    align-content: center;
    justify-items: center;
    gap: var(--space-6);
    min-block-size: 100vh;
    padding: var(--space-6);
  }

  .screen > :global(:not(header)) {
    inline-size: min(100%, 23rem);
  }

  header {
    display: grid;
    justify-items: center;
    gap: var(--space-3);
  }

  h1 {
    margin: 0;
    font-size: var(--text-xl);
    line-height: var(--leading-tight);
  }

  .card {
    box-sizing: border-box;
    display: grid;
    gap: var(--space-4);
    padding: var(--space-6);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  .group {
    display: grid;
    gap: var(--space-3);
  }

  .detail {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .detail code {
    font-family: var(--font-mono);
  }

  button {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-2);
    box-sizing: border-box;
    inline-size: 100%;
    block-size: var(--control-lg);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    font: inherit;
    font-weight: var(--weight-bold);
    cursor: pointer;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover:enabled {
    background: var(--accent-hover);
  }

  button:active:enabled {
    background: var(--accent-pressed);
  }

  /* While the hub answers, the button shows it is off (opacity only, so nothing moves). */
  button:disabled {
    opacity: var(--opacity-disabled);
    cursor: default;
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }
</style>
