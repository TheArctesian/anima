<script lang="ts">
  import { onMount } from 'svelte';
  import { resolve } from '$app/paths';
  import { localHubCarried } from '$lib/desktop/local-hub/shell';
  import Icon from '$lib/icons/Icon.svelte';
  import { resolve as entrance } from '$lib/motion/roles/resolve';
  import { capabilities } from '$lib/platform/platform';
  import { hubChoices } from '../welcome';

  const { localHub, deviceLink, cloudAccount } = capabilities();
  /** Unknown until the shell answers; "Create" stays off until then. */
  let carried = $state(false);
  let title: HTMLElement | undefined = $state();
  const choices = $derived(hubChoices({ deviceLink, cloudAccount, localHub, carried }));

  onMount(() => {
    title?.focus();
    if (localHub) void localHubCarried().then((answer) => (carried = answer));
  });
</script>

<!-- Behind "Join a hub" on the first screen: the ways to reach a hub of one's own (FRN-001, HMD-022). Mockup:
     docs/mockup/first-run/index.html, row 1. -->
<svelte:head><title>Join a hub – Anima.ing</title></svelte:head>

<h1 tabindex="-1" bind:this={title}>Join a hub</h1>

<ul class="choices" {@attach entrance()}>
  {#each choices as choice (choice.id)}
    <li class:off={choice.off !== null}>
      {#if choice.off === null}
        <a href={resolve(choice.href)} aria-describedby="{choice.id}-line">
          <strong>{choice.title}</strong><Icon name="carbon:arrow-right" size="sm" />
        </a>
      {:else}
        <strong>{choice.title}</strong>
      {/if}
      <p id="{choice.id}-line">{choice.line}</p>
      {#if choice.off !== null}<p class="reason">{choice.off}</p>{/if}
    </li>
  {/each}
</ul>

<a class="back" href={resolve('/welcome')}>Back</a>

<style>
  h1:focus-visible,
  a:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
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

  li a {
    display: flex;
    align-items: center;
    justify-content: space-between;
    min-block-size: var(--target-min-desktop);
    color: var(--text);
    text-decoration: none;
  }

  /* Only a pointer that hovers: on a touch screen iOS takes the first tap as a hover, and the link needs two. */
  @media (hover: hover) {
    li a:hover {
      color: var(--accent-hover);
    }
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

  .back {
    display: inline-flex;
    align-items: center;
    justify-self: start;
    min-block-size: var(--target-min-touch);
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }
</style>
