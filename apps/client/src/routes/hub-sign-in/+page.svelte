<script lang="ts">
  import { onMount } from 'svelte';
  import { enterGateway, leftGateway, mayEnterByItself } from '$lib/hosted/gateway/here';
  import { press } from '$lib/motion/roles/press';

  // The sign-in of a hosted hub in a browser (HMD-059, HMD-063): the hub has no password that a person knows (HMD-038),
  // so this page has no form. The entry of the gateway makes the session of the hub again, or leads to the sign-in of
  // the Anima.ing account. The browser goes there by itself once; after a sign-out, or when the entry just failed, the
  // person presses the button.
  const left = leftGateway();
  let going = $state(false);

  function enter(): void {
    going = true;
    enterGateway();
  }

  onMount(() => {
    if (mayEnterByItself()) enter();
  });
</script>

<svelte:head><title>Sign in · Anima.ing</title></svelte:head>

<main>
  <div class="panel">
    {#if left}
      <h1>Signed out</h1>
      <p>This browser is signed out of your hub.</p>
    {:else if going}
      <h1>Signing in</h1>
      <p>Your hub is opening.</p>
    {:else}
      <h1>Sign in again</h1>
      <p>Your hub could not open a session for this browser.</p>
    {/if}
    <button type="button" tabindex="0" disabled={going} onclick={enter} {@attach press()}>{left ? 'Sign in' : 'Try again'}</button>
  </div>
</main>

<style>
  main {
    display: grid;
    place-items: center;
    min-block-size: 100dvh;
    padding: var(--space-6);
    box-sizing: border-box;
  }

  .panel {
    display: grid;
    gap: var(--space-4);
    inline-size: min(100%, calc(var(--space-16) * 5));
  }

  h1 {
    margin: 0;
    font-size: var(--text-display);
    font-weight: var(--weight-light);
    line-height: var(--leading-tight);
    letter-spacing: var(--tracking-tight);
  }

  p {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-md);
  }

  button {
    block-size: var(--control-lg);
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--on-accent);
    font: inherit;
    font-weight: var(--weight-medium);
    cursor: pointer;
  }

  button:disabled {
    opacity: var(--opacity-disabled);
    cursor: default;
  }

  @media (hover: hover) {
    button:not(:disabled):hover {
      background: var(--accent-hover);
    }
  }
</style>
