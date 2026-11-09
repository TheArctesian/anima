<script lang="ts">
  import { tick } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { useHub } from '$lib/hub/context';
  import type { HubResult } from '$lib/hub/hub';
  import { retentionApi, type RetentionPeriod } from './api';
  import { inPageOrder, SUBJECTS } from './periods';
  import RetentionRow from './RetentionRow.svelte';

  const api = retentionApi(useHub());
  /** `null` while loading or after a failed load. */
  let periods = $state<readonly RetentionPeriod[] | null>(null);
  let loadFailed = $state(false);
  let retrying = $state(false);
  let rows: HTMLElement | undefined = $state();

  async function load(): Promise<void> {
    const result = await api.list();
    if (result.ok) {
      periods = inPageOrder(result.value);
      loadFailed = false;
    } else loadFailed = true;
  }

  /** Try again: the button stays mounted, and aria-disabled, while the hub answers; on success focus goes to the first period (NAV-005). */
  async function retry(): Promise<void> {
    if (retrying) return;
    retrying = true;
    try {
      await load();
    } finally {
      retrying = false;
    }
    if (!loadFailed) {
      await tick();
      rows?.querySelector('input')?.focus();
    }
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

<div class="rows" bind:this={rows}>
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
</div>
<!-- One block holds the note or, when the load failed, its message and retry, so the section keeps its height at every width. -->
<div class="note">
  {#if loadFailed}
    <p role="alert">Could not load the periods.</p>
    <FormButton tabindex={0} aria-disabled={retrying} aria-busy={retrying} onclick={retry}>Try again</FormButton>
  {:else}
    <p>Changes apply at the next daily run.</p>
  {/if}
</div>

<style>
  .rows {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    margin-block-start: var(--space-3);
  }

  .note {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    /* The note, and the failure message that wraps to two lines beside Try again, fill the same block (MOT-007). */
    min-block-size: max(var(--target-min-desktop), calc(var(--text-md) * var(--leading-tight) * 2));
    margin-block-start: var(--space-3);
  }

  /* Try again stays on one line, so the failed block keeps the height of the note. */
  .note :global(button) {
    white-space: nowrap;
  }

  p {
    margin: 0;
    font-size: var(--text-md);
  }

  p:not([role]) {
    color: var(--text-muted);
  }

  /* --status-down text is 3.05:1 on the page (MAT-005 needs 4.5:1): the text stays --text and a bar marks the error. */
  [role='alert'] {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }
</style>
