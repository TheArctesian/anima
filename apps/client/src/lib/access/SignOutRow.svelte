<script lang="ts">
  import { tick } from 'svelte';
  import { useHub } from '$lib/hub/context';
  import { clearAccessToken } from '$lib/desktop/token';
  import { press } from '$lib/motion/roles/press';
  import { capabilities } from '$lib/platform/platform';
  import { accessApi } from './api';
  import { signInNeeded } from './guard';
  import { goToSignIn, loadClientAt } from './navigation';
  import { SIGN_IN_PATH } from './routes';
  import { signOut } from './sign-out';

  const hub = useHub();
  const api = accessApi(hub);
  const desktop = capabilities().keyStore;
  let username = $state<string | null>(null);
  let problem = $state<string | null>(null);
  let busy = $state(false);
  let button: HTMLButtonElement | undefined = $state();

  $effect(() => {
    void api.me().then((result) => {
      if (result.ok) username = result.value.username;
      else if (!signInNeeded(result.error, goToSignIn)) problem = 'Anima could not read who is signed in.';
    });
  });

  async function leave(): Promise<void> {
    busy = true;
    let kept: string | null;
    try {
      kept = await signOut({ api, address: hub.address, desktop, clearToken: clearAccessToken });
    } catch (reason) {
      busy = false;
      throw reason;
    }
    if (kept === null && !desktop) {
      // A new page, so no socket keeps the old cookie (E-4); the button stays off while it loads, as Sign in does.
      loadClientAt(SIGN_IN_PATH);
      return;
    }
    busy = false;
    // The desktop socket reads the key store anew, so the router may move within the page.
    if (kept === null) {
      goToSignIn();
      return;
    }
    problem = kept;
    // The disabled button lost focus; give it back once it is enabled again, so the keyboard user stays here.
    await tick();
    button?.focus();
  }
</script>

<div class="row">
  <!-- Always rendered with two lines reserved (the label, then the name on its own line), so the button does not move when the name arrives, however narrow the column is (MOT-007). -->
  <p class="who" class:pending={username === null}>Signed in as <strong>{username ?? ''}</strong></p>
  <!-- tabindex="0": WebKit skips plain buttons when Tab is pressed unless they are focusable, as for the nav links (NAV-005, WP-11 H-6). -->
  <button type="button" tabindex="0" disabled={busy} onclick={leave} bind:this={button} {@attach press()}>Sign out</button>
  <!-- Always rendered and empty when idle: lines are reserved, so a message moves nothing below the row (MOT-007), and a
       live region that is already there is announced more reliably than one that appears filled. -->
  <p role="alert">{problem ?? ''}</p>
</div>

<style>
  .row {
    display: flex;
    flex-direction: column;
    align-items: start;
    gap: var(--space-2);
    margin-block-start: var(--space-6);
  }

  p {
    margin: 0;
    font-size: var(--text-md);
  }

  /* Two lines are reserved; the name never wraps, it is cut with an ellipsis, so its arrival cannot change the height (MOT-007). */
  .who {
    min-block-size: calc(var(--text-md) * var(--leading-tight) * 2);
    max-inline-size: 100%;
    overflow: hidden;
    line-height: var(--leading-tight);
    white-space: nowrap;
  }

  .who.pending {
    visibility: hidden;
  }

  .who strong {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  button {
    box-sizing: border-box;
    block-size: var(--space-8);
    padding: 0 var(--space-4);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    transition:
      background-color var(--duration-fast) var(--ease-enter),
      opacity var(--duration-fast) var(--ease-enter);
  }

  button:hover:enabled {
    background: var(--surface-raised);
  }

  /* While the hub answers, the button shows it is off (opacity only, so nothing moves). */
  button:disabled {
    opacity: var(--opacity-disabled);
    cursor: default;
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  /* --status-down text is 3.05:1 on the page (MAT-005 needs 4.5:1): the text stays --text and a bar marks the error.
     Four lines are reserved: the longest message takes four in Chromium at a 390 px window (MOT-007, MV-005). The bar
     shows only with a message. */
  [role='alert'] {
    min-block-size: calc(4 * var(--text-md) * var(--leading-tight));
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid transparent;
    line-height: var(--leading-tight);
  }

  [role='alert']:not(:empty) {
    border-inline-start-color: var(--status-down);
  }
</style>
