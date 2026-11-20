<script lang="ts">
  import { onMount } from 'svelte';
  import { resolve } from '$app/paths';
  import { resolve as entrance } from '$lib/motion/roles/resolve';
  import Icon from '$lib/icons/Icon.svelte';
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

<svelte:head><title>Welcome – Anima.ing</title></svelte:head>

<!-- The first screen of the first run (FRN-001, HMD-022), as frame 1 of docs/mockup/mobile/index.html: the doors, the
     title, the account actions, and a separate row for joining a hub. Sign up opens the same account page as Sign in. -->
<div class="art" aria-hidden="true"><Doors /></div>

<div class="foot">
  <h1 tabindex="-1" bind:this={title}>Welcome to <span class="wordmark"><span>Anima</span>.ing</span></h1>
  <div class="actions" {@attach entrance()}>
    {#if signIn.off !== null}<p class="reason" id="sign-in-off">{signIn.off}</p>{/if}
    <div class="pair">
      {#if signIn.off === null}
        <a class="button primary" href={resolve(signIn.href)} {@attach press()}>Sign in</a>
        <a class="button secondary" href={resolve(signIn.href)} {@attach press()}>Sign up</a>
      {:else}
        <span class="button primary off" aria-disabled="true" aria-describedby="sign-in-off">Sign in</span>
        <span class="button secondary off" aria-disabled="true" aria-describedby="sign-in-off">Sign up</span>
      {/if}
    </div>
    <a class="button join" href={resolve('/welcome/hub')} aria-describedby="join-hub-description" {@attach press()}>
      <span>Join a hub</span><Icon name="carbon:chevron-right" size="sm" />
    </a>
    <p id="join-hub-description" class="join-description">Connect to a hub your team already runs.</p>
    {#if deviceLink}<a class="button join" href={resolve('/welcome/join')} {@attach press()}>Scan enrollment QR</a>{/if}
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

  .wordmark {
    display: inline-block;
  }

  @supports (background-clip: text) and (color: light-dark(currentColor, currentColor)) {
    @media (prefers-contrast: no-preference) and (forced-colors: none) {
      .wordmark > span {
        background-image: linear-gradient(
          90deg,
          var(--graphite-frost-10),
          light-dark(var(--graphite-accent-deep), var(--graphite-frost-8)),
          light-dark(var(--graphite-accent-deep), var(--graphite-frost-7))
        );
        background-clip: text;
        color: transparent;
      }
    }
  }

  .actions {
    display: grid;
    gap: var(--space-2);
    inline-size: min(100%, calc(var(--space-16) * 4.5));
    margin-block-start: var(--space-4);
  }

  .pair {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-2);
  }

  .button {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: center;
    min-block-size: var(--space-12);
    padding: var(--space-1) var(--space-2);
    border: var(--border-thin) solid transparent;
    border-radius: var(--space-3);
    color: var(--text);
    font-size: var(--text-lg);
    font-weight: var(--weight-medium);
    overflow-wrap: anywhere;
    text-align: center;
    text-decoration: none;
  }

  .primary {
    background: var(--accent);
    color: var(--on-accent);
  }

  .secondary {
    border-color: var(--border-strong);
  }

  .join {
    justify-content: space-between;
    gap: var(--space-2);
    padding-inline: 0 var(--space-1);
    border-block-start-color: var(--border-subtle);
    border-radius: 0;
    color: var(--text-secondary);
    font-weight: var(--weight-regular);
    text-align: start;
  }

  .button.off {
    opacity: var(--opacity-disabled);
  }

  @media (hover: hover) {
    a.primary:hover {
      background: var(--accent-hover);
    }

    a.secondary:hover {
      background: var(--surface-hover);
    }

    .join:hover {
      color: var(--text);
    }
  }

  .reason {
    margin: 0;
    padding-block: var(--space-3) var(--space-1);
    border-block-start: var(--border-thin) solid var(--border-subtle);
    color: var(--text-secondary);
    font-size: var(--text-md);
  }

  .join-description {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  @media (max-height: 42rem) {
    .art {
      max-block-size: 30svh;
    }
  }

  /* On a phone the title and the actions sit low, under the thumb, and the doors fill the room above them (frame 1 of
     the mobile mockup). */
  @media (max-width: 767.98px) {
    .art {
      block-size: auto;
      min-block-size: min(calc(var(--space-16) * 2), 25svh);
    }
  }

  @media (min-width: 767.99px), (min-width: 36rem) and (max-height: 36rem) {
    :global(:root:is([data-apple-mobile], [data-os='android'])) .art {
      block-size: clamp(calc(var(--space-16) * 2), 36svh, calc(var(--space-16) * 4));
      min-block-size: 0;
      margin-inline: 0;
    }
  }
</style>
