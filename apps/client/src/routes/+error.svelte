<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import FormButton from '$lib/forms/FormButton.svelte';
  import EmptyState from '$lib/states/EmptyState.svelte';

  // An address the app has no page for, or a page that failed to open, inside the shell: the same look as every other
  // empty state, and one way on (UI-030).
  const missing = $derived(page.status === 404);
</script>

<section class="error">
  {#if missing}
    <EmptyState level={1} title="Page not found" line="Anima.ing has no page at this address.">
      {#snippet action()}
        <FormButton onclick={() => void goto(resolve('/devices'))}>Go to Devices</FormButton>
      {/snippet}
    </EmptyState>
  {:else}
    <EmptyState level={1} title="This page did not open" line="Something went wrong while it loaded. Reload to try again.">
      {#snippet action()}
        <FormButton onclick={() => window.location.reload()}>Reload</FormButton>
      {/snippet}
    </EmptyState>
  {/if}
</section>

<style>
  .error {
    display: grid;
    place-items: center;
    min-block-size: 60vh;
  }
</style>
