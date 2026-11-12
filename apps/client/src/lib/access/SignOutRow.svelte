<script lang="ts">
  import { tick } from 'svelte';
  import { useHub } from '$lib/hub/context';
  import { clearAccessToken } from '$lib/desktop/token';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { capabilities } from '$lib/platform/platform';
  import SettingRow from '$lib/settings/SettingRow.svelte';
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

<SettingRow>
  {#snippet label()}
    <!-- Always rendered, with its line reserved, so nothing moves when the name arrives (MOT-007); a long name is cut. -->
    <p class="who" class:pending={username === null}>Signed in as <strong>{username ?? ''}</strong></p>
  {/snippet}
  <!-- tabindex="0": WebKit skips plain buttons when Tab is pressed unless they are focusable, as for the nav links (NAV-005, WP-11 H-6). -->
  <button type="button" tabindex="0" disabled={busy} onclick={leave} bind:this={button} {@attach press()}>
    <Icon name="nf-md-logout" size="sm" />Sign out
  </button>
  {#snippet message()}
    <!-- Always rendered and empty when idle: a live region that is already there is announced more reliably than one
         that appears filled, and its line is kept (MOT-007). -->
    <p role="alert">{problem ?? ''}</p>
  {/snippet}
</SettingRow>

<style>
  /* One line, reserved; the name never wraps, it is cut with an ellipsis, so its arrival cannot change the height (MOT-007). */
  .who {
    margin: 0;
    min-block-size: calc(var(--text-md) * var(--leading-tight));
    overflow: hidden;
    font-size: var(--text-md);
    line-height: var(--leading-tight);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .who.pending {
    visibility: hidden;
  }

  button {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    box-sizing: border-box;
    block-size: var(--space-8);
    padding: 0 var(--space-4);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
    font-size: var(--text-md);
    white-space: nowrap;
    transition:
      background-color var(--duration-fast) var(--ease-enter),
      opacity var(--duration-fast) var(--ease-enter);
  }

  button:hover:enabled {
    background: var(--surface-hover);
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
</style>
