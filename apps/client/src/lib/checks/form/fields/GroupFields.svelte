<script lang="ts">
  import type { SiblingCheck } from '../model/group';

  interface Props {
    /** Already filtered by `groupCandidates`. */
    candidates: readonly SiblingCheck[];
    childIds: string[];
    error?: string | undefined;
  }

  let { candidates, childIds = $bindable(), error }: Props = $props();
</script>

<fieldset>
  <legend>Checks in this group</legend>
  {#each candidates as candidate (candidate.id)}
    <label>
      <input type="checkbox" value={candidate.id} bind:group={childIds} />
      <span>{candidate.name}</span>
    </label>
  {:else}
    <p>This device has no other checks to group.</p>
  {/each}
  {#if error !== undefined}<p class="error" role="alert">{error}</p>{/if}
</fieldset>

<style>
  fieldset {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
    padding: var(--space-3);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
  }

  legend {
    padding: 0 var(--space-1);
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }

  label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--target-min-desktop);
  }

  p {
    margin: 0;
    color: var(--text-secondary);
  }

  .error {
    font-size: var(--text-xs);
    color: var(--status-down);
  }
</style>
