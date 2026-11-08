<script lang="ts">
  import { flushSync, untrack } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Modal from '$lib/forms/modal/Modal.svelte';
  import type { ApiKey, ApiKeysApi } from './api';
  import KeySecret from './KeySecret.svelte';
  import { shownDate } from './listing';
  import { checkNewKey, createFailure, dayText, emptyForm, EXPIRY_CHOICES, expiryOf, latestExpiry, SCOPES, type NewKeyProblem } from './new-key';

  interface Props {
    api: Pick<ApiKeysApi, 'create'>;
    now: () => Date;
    /** The key the hub made, without its secret, for the list. */
    onCreated: (key: ApiKey) => void;
    onClose: () => void;
  }

  const { api, now, onCreated, onClose }: Props = $props();
  const ids = $props.id();
  const form = $state(emptyForm());
  let problems = $state<readonly NewKeyProblem[]>([]);
  let failure = $state<string | null>(null);
  let busy = $state(false);
  let created = $state<ApiKey | null>(null);
  /** The whole key, from the hub's answer until the dialog closes; held nowhere else (INT-003, LOG-007). */
  let secret = $state<string | null>(null);

  /** When the dialog opened: the presets' preview and the date field's bounds count from here; the sent date counts from Create. */
  const opened = untrack(() => now());
  const preview = $derived(expiryOf(form, opened));
  const nameProblem = $derived(problems.find((problem) => problem.field === 'name')?.message ?? '');
  const expiryProblem = $derived(problems.find((problem) => problem.field === 'expiry')?.message ?? '');

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (busy || created !== null) return;
    const checked = checkNewKey(form, now());
    failure = null;
    if (!checked.ok) {
      problems = checked.problems;
      document.getElementById(`${ids}-${checked.problems[0]?.field ?? 'name'}`)?.focus();
      return;
    }
    problems = [];
    busy = true;
    const result = await api.create(checked.input);
    busy = false;
    if (!result.ok) {
      failure = createFailure(result.error);
      return;
    }
    onCreated(result.value.key);
    created = result.value.key;
    secret = result.value.secret;
  }

  /**
   * Drops the key from this dialog's state and, in the same task, from the DOM, before the dialog fades (`KeySecret`
   * empties its field as it leaves). Refused while the hub is making the key, so a made key's secret is never lost unseen.
   */
  function close(): void {
    if (busy) return;
    secret = null;
    flushSync();
    onClose();
  }
</script>

<Modal open title={created === null ? 'Create an API key' : `Copy the key ${created.name}`} describedBy="{ids}-about" closeOnScrim={created === null} width={7} onClose={close}>
  {#if created === null}
    <p id="{ids}-about">Programs use an API key to call the hub's GraphQL API. A key cannot make or revoke keys.</p>
    <form onsubmit={submit} novalidate>
      <fieldset class="fields" disabled={busy}>
        <div class="field">
          <label for="{ids}-name">Name</label>
          <input id="{ids}-name" type="text" autocomplete="off" spellcheck="false" bind:value={form.name} aria-invalid={nameProblem !== ''} aria-describedby="{ids}-name-problem" />
          <small id="{ids}-name-problem" class="problem">{nameProblem}</small>
        </div>
        <fieldset class="choice">
          <legend>Scope</legend>
          {#each SCOPES as scope (scope.id)}
            <label><input type="radio" name="{ids}-scope" value={scope.id} bind:group={form.scope} /> {scope.label} <small>{scope.hint}</small></label>
          {/each}
        </fieldset>
        <fieldset class="choice" aria-describedby="{ids}-expiry-problem">
          <legend>Expires</legend>
          {#each EXPIRY_CHOICES as choice (choice.id)}
            <label><input type="radio" name="{ids}-expiry" value={choice.id} bind:group={form.expiry} /> {choice.label}</label>
          {/each}
          <!-- Always laid out, so choosing "On a date" moves nothing (MOT-007). -->
          <input
            id="{ids}-expiry"
            type="date"
            aria-label="Expiry date"
            disabled={form.expiry !== 'date'}
            min={dayText(opened)}
            max={dayText(latestExpiry(opened))}
            bind:value={form.date}
            aria-invalid={expiryProblem !== ''}
          />
          <small>{preview === null ? 'At most one year away.' : `The key stops working on ${shownDate(preview.toISOString())}.`}</small>
          <small id="{ids}-expiry-problem" class="problem">{expiryProblem}</small>
        </fieldset>
        {#if failure !== null}
          <p class="failure" role="alert">{failure}</p>
        {/if}
        <div class="actions">
          <FormButton tabindex={0} onclick={close}>Cancel</FormButton>
          <FormButton tabindex={0} type="submit" tone="primary" aria-busy={busy}>{busy ? 'Creating' : 'Create key'}</FormButton>
        </div>
      </fieldset>
    </form>
  {:else}
    <p id="{ids}-about">Anima shows this key only once. Copy it now and keep it where your program reads it; the hub keeps only a hash of it.</p>
    {#if secret !== null}
      <KeySecret {secret} />
    {/if}
    <div class="actions">
      <FormButton tabindex={0} tone="primary" onclick={close}>Done</FormButton>
    </div>
  {/if}
</Modal>

<style>
  p {
    margin: 0;
    line-height: var(--leading-normal);
  }

  .fields,
  .choice {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-inline-size: 0;
    margin: 0;
    padding: 0;
    border: 0;
  }

  .fields {
    gap: var(--space-3);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  .field > label,
  legend {
    padding: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .choice label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  small {
    color: var(--text-muted);
    font-size: var(--text-xs);
  }

  /* A problem is text with a bar, not colour alone (UI-003). */
  .problem:not(:empty) {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
    color: var(--text);
  }

  .problem:empty {
    display: none;
  }

  input[type='text'],
  input[type='date'] {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: var(--space-2) var(--space-3);
    background-color: var(--surface);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font: inherit;
  }

  input[type='date'] {
    align-self: start;
  }

  input:disabled {
    color: var(--text-muted);
  }

  [aria-invalid='true'] {
    border-color: var(--status-down);
  }

  input:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .failure {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
</style>
