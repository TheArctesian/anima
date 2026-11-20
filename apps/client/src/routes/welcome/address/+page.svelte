<script lang="ts">
  import { resolve } from '$app/paths';
  import { onMount } from 'svelte';
  import { loadClientAt } from '$lib/access/navigation';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { capabilities } from '$lib/platform/platform';
  import { saveHubAddress } from '../../sign-in/sign-in';
  import TextField from '../../sign-in/TextField.svelte';

  const canChooseHub = capabilities().keyStore;
  let title: HTMLElement | undefined = $state();
  let field: HTMLInputElement | undefined = $state();
  let address = $state('');
  let problem = $state<string | undefined>();
  let leaving = $state(false);

  onMount(() => title?.focus());

  function submit(event: SubmitEvent): void {
    event.preventDefault();
    if (!canChooseHub || leaving) return;
    problem = saveHubAddress(localStorage, address, () => {
      leaving = true;
      loadClientAt('/sign-in');
    }) ?? undefined;
    if (problem !== undefined) field?.focus();
  }
</script>

<svelte:head><title>Hub address – Anima.ing</title></svelte:head>

<a class="back" href={resolve('/welcome/hub')}>Back</a>
{#if canChooseHub}
  <h1 tabindex="-1" bind:this={title}>Connect to your hub</h1>
  <form novalidate onsubmit={submit}>
    <TextField id="hub-address" name="hubAddress" label="Hub address" type="url" autocomplete="off"
      bind:value={address} bind:input={field} error={problem} oninput={() => { problem = undefined; }} />
    <FormButton type="submit" tone="primary" size="lg" busy={leaving}>Continue</FormButton>
  </form>
{:else}
  <h1 tabindex="-1" bind:this={title}>Connect to your hub</h1>
  <p>Open your hub’s address in your browser.</p>
  <a href={resolve('/sign-in')}>Sign in to this hub</a>
{/if}

<style>
  form {
    display: grid;
    gap: var(--space-4);
  }

  .back {
    display: inline-flex;
    align-items: center;
    justify-self: start;
    min-block-size: var(--target-min-touch);
    color: var(--text-secondary);
  }

  p {
    margin: 0;
    color: var(--text-secondary);
  }
</style>
