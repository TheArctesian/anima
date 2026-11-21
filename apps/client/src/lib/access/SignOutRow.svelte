<script lang="ts">
  import { ACCOUNT_PAGE, hubIsHosted } from '$lib/hosted/mark';
  import { forgetRecentCopy } from '$lib/hub/hub';
  import { tick } from 'svelte';
  import { useHub } from '$lib/hub/context';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import { accessApi } from './api';
  import { signInNeeded } from './guard';
  import { leaveHub } from './leave/leave';
  import { goToSignIn } from './navigation';

  const hub = useHub();
  const api = accessApi(hub);
  // A hosted hub has no password that a person knows: the way in is the Anima.ing account (HMD-038, HMD-061).
  const hosted = hubIsHosted(hub.address.origin);
  let username = $state<string | null>(null);
  let problem = $state<string | null>(null);
  let busy = $state(false);
  let button: HTMLButtonElement | undefined = $state();

  $effect(() => {
    void api.me().then((result) => {
      if (result.ok) username = result.value.username;
      else if (!signInNeeded(result.error, goToSignIn)) problem = "Can't read who is signed in.";
    });
  });

  async function leave(): Promise<void> {
    busy = true;
    let kept: string | null;
    try {
      kept = await leaveHub(hub, api, forgetRecentCopy);
    } catch (reason) {
      busy = false;
      throw reason;
    }
    // The client is on its way to the sign-in: the button stays off while it goes, as Sign in does.
    if (kept === null) return;
    busy = false;
    problem = kept;
    // The disabled button lost focus; give it back once it is enabled again, so the keyboard user stays here.
    await tick();
    button?.focus();
  }
</script>

<SettingRow>
  {#snippet label()}
    <!-- Always rendered, with its line reserved, so nothing moves when the name arrives (MOT-007); a long name is cut. -->
    <p class="who" class:pending={username === null}>
      Signed in as <strong>{username ?? ''}</strong>{#if hosted}&#32;with your Anima.ing account{/if}
    </p>
    {#if hosted}
      <!-- The hub knows no email of the account (the gateway's ticket names none, HMD-061): the account page has it. -->
      <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- the page of the account on cloud.anima.ing, not a route of this app -->
      <a class="account" href={ACCOUNT_PAGE} target="_blank" rel="noopener noreferrer">Manage account and sign-in methods</a>
    {/if}
  {/snippet}
  <!-- tabindex="0": WebKit skips plain buttons when Tab is pressed unless they are focusable, as for the nav links (NAV-005, WP-11 H-6). -->
  <button type="button" tabindex="0" disabled={busy} onclick={leave} bind:this={button} {@attach press()}>
    <Icon name="carbon:logout" size="sm" />Sign out
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

  .account {
    display: inline-block;
    margin-block-start: var(--space-1);
    color: var(--accent);
    font-size: var(--text-sm);
  }

  .account:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
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
