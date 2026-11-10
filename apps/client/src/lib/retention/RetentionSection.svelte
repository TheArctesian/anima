<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import { useHub } from '$lib/hub/context';
  import Icon from '$lib/icons/Icon.svelte';
  import type { HubResult } from '$lib/hub/hub';
  import { retentionApi, type RetentionPeriod } from './api';
  import { inPageOrder, SUBJECTS } from './periods';
  import RetentionRow from './RetentionRow.svelte';

  const api = retentionApi(useHub());
  /** `null` while loading or after a failed load. */
  let periods = $state<readonly RetentionPeriod[] | null>(null);
  let loadFailed = $state(false);

  async function load(): Promise<void> {
    loadFailed = false;
    const result = await api.list();
    if (result.ok) periods = inPageOrder(result.value);
    else loadFailed = true;
  }

  $effect(() => {
    void load();
  });

  /** Sends one subject's days and, when the hub accepts, shows the period it returns (HUB-005). */
  async function save(subject: RetentionPeriod['subject'], days: number): Promise<HubResult<RetentionPeriod>> {
    const result = await api.set(subject, days);
    if (result.ok) periods = periods?.map((each) => (each.subject === subject ? result.value : each)) ?? null;
    return result;
  }
</script>

{#if periods === null}
  {#each SUBJECTS as { subject, label } (subject)}
    <RetentionRow {label} period={null} />
  {/each}
{:else}
  {#each periods as period (period.subject)}
    <RetentionRow
      label={SUBJECTS.find((each) => each.subject === period.subject)?.label ?? period.subject}
      {period}
      save={(days) => save(period.subject, days)}
    />
  {/each}
{/if}
<!-- The card's footer holds the note or, when the load failed, its notice and Try again, at one height (MOT-007). -->
<div class="note">
  {#if loadFailed}
    <span class="mark"><Icon name="nf-md-alert_circle_outline" size="sm" /></span>
    <p role="alert">Could not load the periods.</p>
    <FormButton tabindex={0} onclick={load}>Try again</FormButton>
  {:else}
    <p>Changes apply at the next daily run.</p>
  {/if}
</div>

<style>
  .note {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    /* The note and the notice beside Try again fill the same block, the height of a button (MOT-007). */
    min-block-size: var(--space-8);
    padding: var(--space-3) var(--space-4);
    border-block-start: var(--border-thin) solid var(--border);
  }

  .note :global(button) {
    white-space: nowrap;
  }

  .mark {
    display: inline-flex;
    color: var(--status-down);
  }

  /* The notice text stays --text (MAT-005); the icon beside it marks the error, never colour alone (UI-003). */
  p {
    flex: 1 1 0;
    min-inline-size: 0;
    margin: 0;
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  p:not([role]) {
    color: var(--text-muted);
  }
</style>
