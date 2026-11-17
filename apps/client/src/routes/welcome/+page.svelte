<script lang="ts">
  import { onMount } from 'svelte';
  import { resolve } from '$app/paths';
  import { localHubCarried } from '$lib/desktop/local-hub/shell';
  import Icon from '$lib/icons/Icon.svelte';
  import { resolve as entrance } from '$lib/motion/roles/resolve';
  import { capabilities } from '$lib/platform/platform';
  import { welcomeChoices } from './welcome';

  const { localHub, deviceLink } = capabilities();
  /** Unknown until the shell answers; "Create" stays off until then. */
  let carried = $state(false);
  let title: HTMLElement | undefined = $state();
  const choices = $derived(welcomeChoices({ deviceLink, localHub, carried }));

  onMount(() => {
    title?.focus();
    if (localHub) void localHubCarried().then((answer) => (carried = answer));
  });
</script>

<h1 tabindex="-1" bind:this={title}>Welcome to Anima.ing</h1>
<p class="lead">Choose how this computer reaches its hub.</p>

<ul class="choices" {@attach entrance()}>
  {#each choices as choice (choice.id)}
    <li class:off={choice.off !== null} class:primary={choice.primary}>
      {#if choice.href !== null && choice.off === null}
        <a href={resolve(choice.href)} aria-describedby="{choice.id}-line">
          <strong>{choice.title}</strong><Icon name="carbon:arrow-right" size="sm" />
        </a>
      {:else}
        <strong>{choice.title}</strong>
      {/if}
      <p id="{choice.id}-line">{choice.line}</p>
      {#if choice.off !== null}<p class="reason">{choice.off}</p>{/if}
      {#if choice.id === 'sign-in'}
        <!-- For an operator with a hub of their own on a server (VPS mode). -->
        <a class="secondary" href={resolve('/sign-in')}>Use a hub address</a>
      {/if}
    </li>
  {/each}
</ul>

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

  .choices {
    display: grid;
    gap: var(--space-3);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  li {
    display: grid;
    gap: var(--space-1);
    padding: var(--space-4);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  li a:first-child {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-block-size: var(--target-min-desktop);
    color: var(--text);
    text-decoration: none;
  }

  li a:first-child:hover {
    color: var(--accent-hover);
  }

  /* The primary choice (HMD-022) carries the accent edge; the others stay plain. */
  .primary:not(.off) {
    border-color: var(--accent);
  }

  .off strong {
    opacity: var(--opacity-disabled);
  }

  p {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  .secondary {
    display: inline-flex;
    align-items: center;
    min-block-size: var(--target-min-desktop);
    justify-self: start;
    color: var(--accent);
    font-size: var(--text-sm);
  }
</style>
