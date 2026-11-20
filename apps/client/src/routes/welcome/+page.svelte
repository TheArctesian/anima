<script lang="ts">
  import { onMount } from 'svelte';
  import { resolve } from '$app/paths';
  import { resolve as entrance } from '$lib/motion/roles/resolve';
  import { press } from '$lib/motion/roles/press';
  import { capabilities } from '$lib/platform/platform';
  import Doors from '../sign-in/doors/Doors.svelte';
  import { signInChoice } from './welcome';

  const { localHub, deviceLink, cloudAccount } = capabilities();
  // Only the hosted sign-in decides this screen; the build's hub programs matter on the page behind "Join a hub".
  const signIn = $derived(signInChoice({ deviceLink, cloudAccount, localHub, carried: false }));
  let title: HTMLElement | undefined = $state();

  onMount(() => {
    title?.focus();
  });
</script>

<!-- The first screen of the first run (FRN-001, HMD-022), as frame 1 of docs/mockup/mobile/index.html: the doors, the
     title, Sign in with the Anima.ing account, then Sign up and Join a hub side by side (operator, 2026-11-17). Sign up
     opens the same account page as Sign in, where a new account is made. -->
<div class="art" aria-hidden="true"><Doors /></div>

<div class="foot">
  <h1 tabindex="-1" bind:this={title}>Welcome to Anima.ing</h1>
  <div class="actions" {@attach entrance()}>
    {#if signIn.off !== null}<p class="reason" id="sign-in-off">{signIn.off}</p>{/if}
    {#if signIn.off === null}
      <a class="button primary" href={resolve(signIn.href)} {@attach press()}>Sign in</a>
    {:else}
      <span class="button primary off" aria-disabled="true" aria-describedby="sign-in-off">Sign in</span>
    {/if}
    <div class="pair">
      {#if signIn.off === null}
        <a class="button" href={resolve(signIn.href)} {@attach press()}>Sign up</a>
      {:else}
        <span class="button off" aria-disabled="true" aria-describedby="sign-in-off">Sign up</span>
      {/if}
      <a class="button" href={resolve('/welcome/hub')} {@attach press()}>Join a hub</a>
    </div>
  </div>
</div>

<style>
  h1:focus-visible,
  a:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }

  /* The doors stand above the title, as tall as the mockup draws them. */
  .art {
    position: relative;
    block-size: calc(var(--space-16) * 5);
    margin-inline: calc(var(--space-4) * -1);
  }

  .actions {
    display: grid;
    gap: calc(var(--space-2) + var(--border-focus));
    margin-block-start: var(--space-6);
  }

  .pair {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: calc(var(--space-2) + var(--border-focus));
  }

  .button {
    display: grid;
    place-items: center;
    min-block-size: var(--space-12);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font-size: var(--text-lg);
    font-weight: var(--weight-medium);
    text-decoration: none;
  }

  .button.primary {
    min-block-size: calc(var(--space-12) + var(--space-2));
    background: var(--accent);
    color: var(--on-accent);
  }

  .button.off {
    opacity: var(--opacity-disabled);
  }

  /* Only a pointer that hovers: on a touch screen iOS takes the first tap as a hover. */
  @media (hover: hover) {
    a.button:hover {
      background: var(--surface-hover);
    }

    a.button.primary:hover {
      background: var(--accent-hover);
    }
  }

  .reason {
    margin: 0;
    padding-block: var(--space-3) var(--space-1);
    border-block-start: var(--border-thin) solid var(--border-subtle);
    color: var(--text-secondary);
    font-size: var(--text-md);
  }

  /* On a phone the title and the actions sit low, under the thumb, and the doors fill the room above them (frame 1 of
     the mobile mockup). */
  @media (max-width: 767.98px) {
    .art {
      block-size: max(calc(var(--space-16) * 3), calc(100dvh - var(--safe-block-start) - var(--safe-block-end) - var(--space-16) * 6.5));
    }

    .foot {
      position: fixed;
      inset-inline: var(--space-6);
      inset-block-end: calc(var(--safe-block-end) + var(--space-3));
    }
  }
</style>
