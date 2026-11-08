<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import type { HubResult } from '$lib/hub/hub';
  import Field from '../Field.svelte';
  import type { NotificationSettings, NotificationSettingsInput } from '../api';
  import { failureText } from '../failure';
  import { D21_PLACEHOLDERS, templateFine, templateProblems, type TemplateProblems } from '../template/template';
  import TemplateFields from '../template/TemplateFields.svelte';
  import { checkDays, daysText, DEFAULT_DAYS } from './days';

  interface Props {
    /** `null` while the hub has not answered: the form keeps its size and its controls wait. */
    settings: NotificationSettings | null;
    loadFailed: boolean;
    onRetry: () => void;
    save: (input: NotificationSettingsInput) => Promise<HubResult<NotificationSettings>>;
  }

  const { settings, loadFailed, onRetry, save }: Props = $props();
  const id = $props.id();
  let days = $state('');
  let template = $state({ title: '', body: '' });
  let daysProblem = $state('');
  let problems = $state<TemplateProblems>({ title: '', body: '' });
  let outcome = $state('');
  let busy = $state(false);
  const waiting = $derived(settings === null);
  const placeholders = $derived(settings?.placeholders ?? D21_PLACEHOLDERS);

  // The fields follow the hub's settings when they arrive or are saved; a refused save leaves what the operator typed.
  $effect(() => {
    if (settings === null) return;
    days = daysText(settings.certificateDays);
    template.title = settings.template.title;
    template.body = settings.template.body;
  });

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (waiting || busy) return;
    const checked = checkDays(days);
    const found = templateProblems(template, placeholders);
    daysProblem = checked.ok ? '' : checked.message;
    problems = found;
    outcome = '';
    if (!checked.ok || !templateFine(found)) return;
    busy = true;
    const result = await save({ certificateDays: checked.days, template: { title: template.title, body: template.body } });
    busy = false;
    outcome = result.ok ? 'Saved.' : failureText(result.error, 'settings');
  }
</script>

<form class="settings" novalidate onsubmit={submit} aria-busy={waiting}>
  <fieldset disabled={waiting || busy}>
    <Field
      label="Certificate expiry days"
      hint="A notice when a certificate or domain has fewer days left: up to 5 values from 1 to 365, largest first. Default {daysText(DEFAULT_DAYS)}."
      problem={daysProblem}
    >
      {#snippet control(attributes)}
        <input {...attributes} type="text" bind:value={days} autocomplete="off" spellcheck="false" />
      {/snippet}
    </Field>
    <fieldset class="template">
      <legend>Default template</legend>
      <TemplateFields bind:template {problems} {placeholders} />
    </fieldset>
  </fieldset>
  <!-- One block holds the outcome or, when the load failed, its message and retry, so the form keeps its height (MOT-007). -->
  <div class="note">
    {#if loadFailed}
      <p id="{id}-failed" role="alert">Anima could not load the notification settings.</p>
      <FormButton tabindex={0} onclick={onRetry}>Try again</FormButton>
    {:else}
      <p class="outcome" role="status">{outcome}</p>
      <FormButton tabindex={0} type="submit" tone="primary" aria-disabled={waiting || busy} aria-busy={busy}>Save</FormButton>
    {/if}
  </div>
</form>

<style>
  .settings,
  fieldset {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-inline-size: 0;
    margin: 0;
    padding: 0;
    border: 0;
  }

  .template {
    gap: var(--space-2);
  }

  legend {
    padding: 0;
    margin-block-end: var(--space-2);
    font-size: var(--text-md);
    font-weight: var(--weight-bold);
  }

  .note {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-2);
    /* The outcome, and the failure message that wraps to two lines beside its button, fill the same block (MOT-007). */
    min-block-size: max(var(--target-min-desktop), calc(var(--text-sm) * var(--leading-normal) * 2));
  }

  .note p {
    margin: 0;
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  .note :global(button) {
    flex: none;
    white-space: nowrap;
  }

  /* --status-down text is 3.05:1 on the page (MAT-005 needs 4.5:1): the text stays --text and a bar marks the error. */
  [role='alert'] {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }
</style>
