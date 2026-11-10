<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import type { HubResult } from '$lib/hub/hub';
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
      result = await save(checked.days);
    } finally {
      busy = false;
    }
    if (!result.ok) problem = `Anima could not save the ${label.toLowerCase()} period.`;
  }
</script>

<!-- A group, not a form landmark: five named forms would be five landmarks; Enter still submits. -->
<form class="row" role="group" novalidate onsubmit={submit} aria-label={label}>
  <label for="{id}-days">{label}</label>
  <div class="control">
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
    <FormButton tabindex={0} type="submit" aria-label="Save {label}" aria-disabled={period === null || busy}>Save</FormButton>
  </div>
  <!-- Two lines in every state, so the answer changes no height at any width; the placeholder has the same two lines, hidden. -->
  <p class="hint" id="{id}-hint" class:waiting={period === null}>
    <span>{period === null ? 'Keeps 00 days.' : `Keeps ${daysText(period.days)}.`}</span>
    <span>{period === null ? '00 to 000 days, default 00' : `${String(period.minDays)} to ${String(period.maxDays)} days, default ${String(period.defaultDays)}`}</span>
  </p>
  <div class="problem" id="{id}-problem" role="alert">{problem}</div>
</form>

<style>
  .row {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  /* The label's line is reserved from tokens, so a font load cannot move what is below it (MOT-007). */
  label {
    min-block-size: calc(var(--text-md) * var(--leading-tight));
    font-size: var(--text-md);
    line-height: var(--leading-tight);
  }

  .control {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  input {
    box-sizing: border-box;
    inline-size: calc(var(--space-8) * 2);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-2);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  input:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  input[aria-invalid='true'] {
    border-color: var(--status-down);
  }

  .unit {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  /* The hint and the message keep their lines whether or not they have text, so no row below moves (MOT-007, MV-005). */
  .hint,
  .problem {
    margin: 0;
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  .hint span {
    display: block;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    min-block-size: calc(var(--text-sm) * var(--leading-tight));
  }

  /* The message may wrap to a second line at a narrow width, so two lines are reserved. */
  .problem {
    min-block-size: calc(var(--text-sm) * var(--leading-tight) * 2);
  }

  .hint {
    color: var(--text-muted);
  }

  .waiting {
    visibility: hidden;
  }

  /* --status-down text is 3.05:1 on the page (MAT-005 needs 4.5:1): the text stays --text and a bar marks the error. */
  .problem:not(:empty) {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }
</style>
