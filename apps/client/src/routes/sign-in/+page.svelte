<script lang="ts">
  import { accessApi } from '$lib/access/api';
  import { goToDevices, reloadClient } from '$lib/access/navigation';
  import { keyStoreAvailable, writeAccessToken } from '$lib/desktop/token';
  import { useHub } from '$lib/hub/context';
  import { press } from '$lib/motion/roles/press';
  import { afterEdit, firstInvalid, type FieldMessages, type LoginField } from './form';
  import { saveHubAddress, signIn } from './sign-in';

  const hub = useHub();
  const api = accessApi(hub);

  let username = $state('');
  let password = $state('');
  let totpCode = $state('');
  let fields = $state<FieldMessages>({});
  let message = $state<string | null>(null);
  let busy = $state(false);
  const inputs: Partial<Record<LoginField, HTMLInputElement>> = {};

  let addressText = $state(hub.address.origin);
  let addressProblem = $state<string | null>(null);

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const form = { username, password, totpCode };
    // The secrets leave the page state as soon as they are read; only the user name stays (LOG-007).
    password = '';
    totpCode = '';
    busy = true;
    const outcome = await signIn({ api, address: hub.address, desktop: keyStoreAvailable(), writeToken: writeAccessToken }, form);
    busy = false;
    if (outcome.signedIn) {
      goToDevices();
      return;
    }
    message = outcome.message;
    fields = outcome.fields;
    const first = firstInvalid(fields);
    if (first !== undefined) inputs[first]?.focus();
  }

  function edited(field: LoginField, event: Event & { currentTarget: HTMLInputElement }): void {
    fields = afterEdit(fields, field, event.currentTarget.value);
  }

  function saveAddress(event: SubmitEvent): void {
    event.preventDefault();
    addressProblem = saveHubAddress(localStorage, addressText, reloadClient);
  }
</script>

<h1>Sign in</h1>

<form onsubmit={submit} novalidate>
  <label for="username">User name</label>
  <input id="username" name="username" autocomplete="username" bind:value={username} aria-describedby="username-problem"
    aria-invalid={fields.username === undefined ? undefined : 'true'}
    bind:this={inputs.username}
    oninput={(event) => {
      edited('username', event);
    }} />
  <p id="username-problem" class="problem">{fields.username ?? ''}</p>

  <label for="password">Password</label>
  <input id="password" name="password" type="password" autocomplete="current-password" bind:value={password} aria-describedby="password-problem"
    aria-invalid={fields.password === undefined ? undefined : 'true'}
    bind:this={inputs.password}
    oninput={(event) => {
      edited('password', event);
    }} />
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
    aria-invalid={fields.totpCode === undefined ? undefined : 'true'}
    bind:this={inputs.totpCode}
    oninput={(event) => {
      edited('totpCode', event);
    }}
  />
  <p id="totp-problem" class="problem">{fields.totpCode ?? ''}</p>

  <!-- tabindex="0": WebKit skips plain buttons when Tab is pressed unless they are focusable, as for the nav links (NAV-005, WP-11 H-6). -->
  <button type="submit" tabindex="0" disabled={busy} {@attach press()}>Sign in</button>
  <p role="alert" class="problem form-problem">{message ?? ''}</p>
</form>

<form onsubmit={saveAddress} novalidate>
  <label for="hub-address">Hub address</label>
  <input id="hub-address" name="hubAddress" autocomplete="off" bind:value={addressText} aria-describedby="address-problem" />
  <p id="address-problem" class="problem">{addressProblem ?? ''}</p>
  <button type="submit" tabindex="0" {@attach press()}>Save address</button>
</form>

<style>
  h1 {
    margin: 0 0 var(--space-4);
    font-size: var(--text-2xl);
    line-height: var(--leading-tight);
    min-block-size: calc(var(--text-2xl) * var(--leading-tight));
  }

  /* The line height is reserved from tokens, so the font swap cannot move the fields below (MOT-007). */
  label {
    font-size: var(--text-md);
    line-height: var(--leading-tight);
    min-block-size: calc(var(--text-md) * var(--leading-tight));
  }

  form {
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    gap: var(--space-2);
    max-inline-size: 24rem;
    margin-block-end: var(--space-8);
  }

  /* A fixed control height from a space token, so the font swap does not grow the controls and move the forms (MOT-007). */
  input,
  button {
    box-sizing: border-box;
    block-size: var(--space-8);
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
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid transparent;
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  /* --status-down text on the page is 3.05:1 (MAT-005 needs 4.5:1 for small text): the text stays --text, a bar marks the error. */
  .problem:not(:empty) {
    border-inline-start-color: var(--status-down);
  }

  /* Two lines, so a message that wraps does not push the hub address form down (MOT-007). */
  .form-problem {
    min-block-size: calc(2 * var(--text-sm) * var(--leading-tight));
  }
</style>
