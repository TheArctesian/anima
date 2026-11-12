<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import { useActionFeedback } from '$lib/forms/feedback/feedback.svelte';
  import type { HubResult } from '$lib/hub/hub';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import type { RetentionPeriod } from './api';
  import { checkDays, daysText } from './periods';

  interface Props {
    label: string;
    /** `null` while the hub has not answered: the row keeps its size and its controls wait. */
    period: RetentionPeriod | null;
    /** Absent while there is no period to save. */
    save?: (days: number) => Promise<HubResult<RetentionPeriod>>;
  }

  const { label, period, save }: Props = $props();
  const id = $props.id();
  let draft = $state('');
  let problem = $state('');
  let busy = $state(false);
  const saved = useActionFeedback();

  // The field follows the hub's days when they arrive or change; a refused save leaves what the operator typed.
  $effect(() => {
    if (period !== null) draft = String(period.days);
  });

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (period === null || save === undefined || busy) return;
    const checked = checkDays(draft, period);
    if (!checked.ok) {
      problem = checked.message;
      return;
    }
    problem = '';
    busy = true;
    let result;
    try {
      result = await saved.track(save(checked.days), (outcome) => outcome.ok);
    } finally {
      busy = false;
    }
    if (!result.ok) problem = `Anima could not save the ${label.toLowerCase()} period.`;
  }
</script>

<!-- A group, not a form landmark: four named forms would be four landmarks; Enter still submits. -->
<form role="group" novalidate onsubmit={submit} aria-label={label}>
  <SettingRow {label} for="{id}-days">
    {#snippet hint()}
      <!-- Two lines in every state, so the answer changes no height; the placeholder has the same two lines, hidden. -->
      <p class="hint" id="{id}-hint" class:waiting={period === null}>
        <span>{period === null ? 'Keeps 00 days.' : `Keeps ${daysText(period.days)}.`}</span>
        <span>{period === null ? '00 to 000 days, default 00' : `${String(period.minDays)} to ${String(period.maxDays)} days, default ${String(period.defaultDays)}`}</span>
      </p>
    {/snippet}
    <input
      id="{id}-days"
      type="number"
      inputmode="numeric"
      step="1"
      min={period?.minDays}
      max={period?.maxDays}
      disabled={period === null}
      aria-invalid={problem === '' ? undefined : true}
      aria-describedby={period === null ? undefined : `${id}-hint ${id}-problem`}
      value={draft}
      oninput={(event) => {
        draft = event.currentTarget.value;
        problem = '';
      }}
    />
    <span class="unit">days</span>
    <FormButton tabindex={0} type="submit" aria-label="Save {label}" aria-disabled={period === null || busy} feedback={saved.state}>Save</FormButton>
    {#snippet message()}
      <!-- One line is kept whether or not it has text, so no row below moves (MOT-007, MV-005). -->
      <div class="problem" id="{id}-problem" role="alert">{problem}</div>
    {/snippet}
  </SettingRow>
</form>

<style>
  .unit {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .hint span {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-block-size: calc(var(--text-sm) * var(--leading-tight));
  }

  .waiting {
    visibility: hidden;
  }
</style>
