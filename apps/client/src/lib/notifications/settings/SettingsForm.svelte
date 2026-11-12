<script lang="ts">
  import { flushSync, tick } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import type { HubResult } from '$lib/hub/hub';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import type { NotificationSettings, NotificationSettingsInput } from '../api';
  import { failureText } from '../failure';
  import { D21_PLACEHOLDERS, templateFine, templateProblems, type TemplateProblems } from '../template/template';
  import { checkThresholds, thresholdsText, DEFAULT_DAYS } from './days';

  interface Props {
    /** `null` while the hub has not answered: the form keeps its size and its controls wait. */
    settings: NotificationSettings | null;
    loadFailed: boolean;
    onRetry: () => Promise<void>;
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
  let retrying = $state(false);
  let form: HTMLFormElement | undefined = $state();
  const waiting = $derived(settings === null);
  const placeholders = $derived(settings?.placeholders ?? D21_PLACEHOLDERS);

  // The fields follow the hub's settings when they arrive or are saved; a refused save leaves what the operator typed.
  $effect(() => {
    if (settings === null) return;
    days = thresholdsText(settings.certificateDays);
    template.title = settings.template.title;
    template.body = settings.template.body;
  });

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (waiting || busy) return;
    const checked = checkThresholds(days);
    const found = templateProblems(template, placeholders);
    daysProblem = checked.ok ? '' : checked.message;
    problems = found;
    outcome = '';
    if (!checked.ok || !templateFine(found)) {
      // Focus goes to the first refused field, which reads out its problem (F-9, UI-015).
      flushSync();
      (event.currentTarget as HTMLFormElement).querySelector<HTMLElement>('[aria-invalid="true"]')?.focus();
      return;
    }
    busy = true;
    const result = await save({ certificateDays: checked.days, template: { title: template.title, body: template.body } });
    busy = false;
    outcome = result.ok ? 'Saved.' : failureText(result.error, 'settings');
  }

  /** Try again: the button stays mounted, and aria-disabled, while the hub answers; on success focus goes to the first field (NAV-005). */
  async function retry(): Promise<void> {
    if (retrying) return;
    retrying = true;
    try {
      await onRetry();
    } finally {
      retrying = false;
    }
    if (!loadFailed) {
      await tick();
      form?.querySelector('input')?.focus();
    }
  }
</script>

<form bind:this={form} class="settings" novalidate onsubmit={submit} aria-busy={waiting}>
  <fieldset disabled={waiting || busy}>
    <SettingRow label="Certificate expiry days" for="{id}-days">
      {#snippet hint()}
        <p class="hint" id="{id}-days-hint">
          A notice when a certificate or domain has fewer days left: up to 5 values from 1 to 365, largest first. Default {thresholdsText(DEFAULT_DAYS)}.
        </p>
      {/snippet}
      <input
        id="{id}-days"
        class="wide"
        type="text"
        bind:value={days}
        autocomplete="off"
        spellcheck="false"
        aria-invalid={daysProblem !== ''}
        aria-describedby="{id}-days-hint {id}-days-problem"
      />
      {#snippet message()}
        <p id="{id}-days-problem" role="alert">{daysProblem}</p>
      {/snippet}
    </SettingRow>
    <h4>Default template</h4>
    <SettingRow label="Title" for="{id}-title">
      {#snippet hint()}
        <p class="hint" id="{id}-title-hint">Placeholders: {placeholders.join(' ')}</p>
      {/snippet}
      <input
        id="{id}-title"
        class="wide"
        type="text"
        bind:value={template.title}
        autocomplete="off"
        aria-invalid={problems.title !== ''}
        aria-describedby="{id}-title-hint {id}-title-problem"
      />
      {#snippet message()}
        <p id="{id}-title-problem" role="alert">{problems.title}</p>
      {/snippet}
    </SettingRow>
    <SettingRow label="Body" for="{id}-body">
      <textarea
        id="{id}-body"
        class="wide"
        bind:value={template.body}
        rows="3"
        autocomplete="off"
        aria-invalid={problems.body !== ''}
        aria-describedby="{id}-body-problem"
      ></textarea>
      {#snippet message()}
        <p id="{id}-body-problem" role="alert">{problems.body}</p>
      {/snippet}
    </SettingRow>
  </fieldset>
  <!-- One row holds the outcome or, when the load failed, its message and retry, so the form keeps its height (MOT-007). -->
  <SettingRow>
    {#snippet label()}
      <div class="note">
        {#if loadFailed}
          <p id="{id}-failed" role="alert">Anima could not load the notification settings.</p>
        {:else}
          <p class="outcome" role="status">{outcome}</p>
        {/if}
      </div>
    {/snippet}
    {#if loadFailed}
      <FormButton tabindex={0} aria-disabled={retrying} aria-busy={retrying} onclick={retry}>Try again</FormButton>
    {:else}
      <FormButton tabindex={0} type="submit" tone="primary" aria-disabled={waiting || busy} aria-busy={busy}>Save</FormButton>
    {/if}
  </SettingRow>
</form>

<style>
  .settings,
  fieldset {
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
    margin: 0;
    padding: 0;
    border: 0;
  }

  /* The sub-heading is a row of its own, inset like the rows (SET-001). */
  h4 {
    margin: 0;
    padding: var(--space-3) var(--space-4);
    border-block-start: var(--border-thin) solid var(--border-strong);
    font-size: var(--text-md);
    font-weight: var(--weight-bold);
    line-height: var(--leading-tight);
  }

  /* Text controls are wider than a number's: they hold a sentence, and shrink with a narrow card. */
  .settings .wide {
    box-sizing: border-box;
    inline-size: calc(var(--space-8) * 8);
    max-inline-size: 100%;
    text-align: start;
  }

  .settings textarea.wide {
    padding: var(--space-2);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background-color: var(--surface-raised);
    color: var(--text);
    font: inherit;
    font-size: var(--text-md);
    resize: vertical;
  }

  .settings textarea.wide:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .settings textarea.wide[aria-invalid='true'] {
    border-color: var(--status-down);
  }

  .settings textarea.wide:disabled {
    color: var(--text-secondary);
  }

  /* Two lines are kept for the outcome and for the failure message, so a retry moves nothing (MOT-007). */
  .note p {
    margin: 0;
    min-block-size: calc(var(--text-sm) * var(--leading-tight) * 2);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  /* --status-down text is 3.05:1 on the page (MAT-005 needs 4.5:1): the text stays --text and a bar marks the error. */
  .note [role='alert'] {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }
</style>
