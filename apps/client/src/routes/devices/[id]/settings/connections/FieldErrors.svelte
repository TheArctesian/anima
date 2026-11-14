<script lang="ts">
  import { errorId } from '$lib/devices/form/form/errors';
  import type { FieldError } from '$lib/hub/hub';
  import Icon from '$lib/icons/Icon.svelte';

  const { errors }: { errors: readonly FieldError[] } = $props();
</script>

<!-- Why a protocol's change was not saved: one full-width line under its fields, so a narrow column is never squeezed
     (D17). The region is always there, so a screen reader hears a reason when it appears. -->
<div class="errors" role="alert">
  {#each errors as error (error.field + error.message)}
    <p id={errorId(error.field)}><Icon name="carbon:warning-alt" size="sm" />{error.message}</p>
  {/each}
</div>

<style>
  .errors {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    grid-column: 1 / -1;
  }

  .errors:empty {
    display: none;
  }

  p {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    color: var(--status-down-text);
    font-size: var(--text-sm);
  }
</style>
