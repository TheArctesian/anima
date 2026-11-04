<script lang="ts">
  import { accessApi } from '$lib/access/api';
  import { goToDevices, reloadClient } from '$lib/access/navigation';
  import { writeAccessToken } from '$lib/desktop/token';
  import { useHub } from '$lib/hub/context';
  import { press } from '$lib/motion/roles/press';
  import type { FieldMessages } from './form';
  import { saveHubAddress, signIn } from './sign-in';

  const hub = useHub();
  const api = accessApi(hub);

  let username = $state('');
  let password = $state('');
  let totpCode = $state('');
  let fields = $state<FieldMessages>({});
  let message = $state<string | null>(null);
  let busy = $state(false);

  let addressText = $state(hub.address.origin);
  let addressProblem = $state<string | null>(null);

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const form = { username, password, totpCode };
    // The secrets leave the page state as soon as they are read; only the user name stays (LOG-007).
    password = '';
    totpCode = '';
    busy = true;
    const outcome = await signIn({ api, address: hub.address, writeToken: writeAccessToken }, form);
    busy = false;
    if (outcome.signedIn) {
      goToDevices();
      return;
    }
    message = outcome.message;
    fields = outcome.fields;
  }

  function saveAddress(event: SubmitEvent): void {
    event.preventDefault();
    addressProblem = saveHubAddress(localStorage, addressText, reloadClient);
  }
</script>

<h1>Sign in</h1>

<form onsubmit={submit} novalidate>
  <label for="username">User name</label>
  <input id="username" name="username" autocomplete="username" bind:value={username} aria-describedby="username-problem" />
  <p id="username-problem" class="problem">{fields.username ?? ''}</p>

  <label for="password">Password</label>
  <input id="password" name="password" type="password" autocomplete="current-password" bind:value={password} aria-describedby="password-problem" />
  <p id="password-problem" class="problem">{fields.password ?? ''}</p>

  <label for="totp">Code from your authenticator</label>
  <input
    id="totp"
    name="totpCode"
    inputmode="numeric"
    autocomplete="one-time-code"
    maxlength="6"
    bind:value={totpCode}
    aria-describedby="totp-problem"
  />
  <p id="totp-problem" class="problem">{fields.totpCode ?? ''}</p>

  <button type="submit" disabled={busy} {@attach press()}>Sign in</button>
  <p role="alert" class="problem">{message ?? ''}</p>
</form>

<form onsubmit={saveAddress} novalidate>
  <label for="hub-address">Hub address</label>
  <input id="hub-address" name="hubAddress" autocomplete="off" bind:value={addressText} aria-describedby="address-problem" />
  <p id="address-problem" class="problem">{addressProblem ?? ''}</p>
  <button type="submit" {@attach press()}>Save address</button>
</form>

<style>
  h1 {
    margin: 0 0 var(--space-4);
    font-size: var(--text-2xl);
    line-height: var(--leading-tight);
  }

  form {
    display: grid;
    gap: var(--space-2);
    max-inline-size: 24rem;
    margin-block-end: var(--space-8);
  }

  input,
  button {
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  button {
    transition: background-color var(--duration-press) var(--ease-enter);
  }

  button:hover {
    background: var(--surface-raised);
  }

  input:focus-visible,
  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .problem {
    margin: 0;
    min-block-size: calc(var(--text-sm) * var(--leading-tight));
    color: var(--status-down);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }
</style>
