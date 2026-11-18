<script lang="ts">
  import { onMount } from 'svelte';
  import { resolve } from '$app/paths';
  import { resolve as entrance } from '$lib/motion/roles/resolve';
  import { press } from '$lib/motion/roles/press';
  import { capabilities } from '$lib/platform/platform';
  import { signInChoice } from './welcome';

  const { localHub, deviceLink, cloudAccount, keyStore } = capabilities();
  // Only the hosted sign-in decides this screen; the build's hub programs matter on the page behind "Join a hub".
  const signIn = $derived(signInChoice({ deviceLink, cloudAccount, localHub, carried: false }, keyStore));
  let title: HTMLElement | undefined = $state();

  onMount(() => {
    title?.focus();
  });
</script>

<!-- The first screen of the first run (FRN-001, HMD-022): sign in with the Anima.ing account, and one link to the other
     ways to reach a hub (operator, 2026-11-17). Mockup: docs/mockup/first-run/index.html, row 1. -->
<h1 tabindex="-1" bind:this={title}>Welcome to Anima.ing</h1>
<p class="lead" id="sign-in-line">{signIn.line}</p>

<div class="actions" {@attach entrance()}>
  {#if signIn.off === null}
    <a class="primary" href={resolve(signIn.href)} aria-describedby="sign-in-line" {@attach press()}>{signIn.title}</a>
  {:else}
    <span class="primary off" aria-disabled="true" aria-describedby="sign-in-off">{signIn.title}</span>
    <p class="reason" id="sign-in-off">{signIn.off}</p>
  {/if}
  <a class="other" href={resolve('/welcome/hub')}>Join a hub</a>
</div>

<style>
  h1:focus-visible,
  a:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
  }

  .lead {
    margin: 0;
    color: var(--text-secondary);
  }

  .actions {
    display: grid;
    gap: var(--space-2);
    margin-block-start: var(--space-4);
  }

  .primary {
    display: grid;
    place-items: center;
    min-block-size: var(--target-min-touch);
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    font-weight: var(--weight-medium);
    text-decoration: none;
  }

  /* Only a pointer that hovers: on a touch screen iOS takes the first tap as a hover. */
  @media (hover: hover) {
    .primary:hover {
      background: var(--accent-hover);
    }
  }

  .primary.off {
    opacity: var(--opacity-disabled);
  }

  .reason {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    text-align: center;
  }

  .other {
    display: grid;
    place-items: center;
    min-block-size: var(--target-min-touch);
    color: var(--accent);
    font-size: var(--text-sm);
  }

  /* On a phone the action sits low, under the thumb (mockup 1 of mobile/index.html); the text stays at the top. */
  @media (max-width: 767.98px) {
    .actions {
      position: fixed;
      inset-inline: var(--space-4);
      inset-block-end: calc(var(--safe-block-end) + var(--space-4));
      margin: 0;
    }
  }
</style>
