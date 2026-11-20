<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import FormButton from '$lib/forms/FormButton.svelte';
  import EmptyState from '$lib/states/EmptyState.svelte';
</script>

<!-- The page of a device the hub does not have: what happened, and the one way on (UI-030). -->
<EmptyState level={1} title="Device not found" line="It may have been removed, or the address is wrong.">
  {#snippet action()}
    <FormButton onclick={() => void goto(resolve('/devices'))}>Go to Devices</FormButton>
  {/snippet}
</EmptyState>
