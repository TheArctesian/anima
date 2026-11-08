<script lang="ts">
  import { fieldErrorsFrom } from '../field-errors';
  import type { FieldError, HubResult } from '$lib/hub/error';
  import FormButton from '../../../forms/FormButton.svelte';
  import type { WebInterface, WebInterfaceInput } from '$lib/web-interfaces/api';
  import { OPEN_IN_CHOICES, validateWeb, webChanged, webInput, webValuesFrom } from './interface';

  interface Props {
    /** The stored interface; absent for the row that adds one. */
    saved?: WebInterface;
    /** A unique prefix for the input ids. */
    rowKey: string;
    onSave: (input: WebInterfaceInput) => Promise<HubResult<unknown>>;
    /** Asks the owner to confirm and delete; absent for the row that adds one. */
    onDelete?: () => void;
    /** Closes the row that adds one. */
    onCancel?: () => void;
  }

  const { saved, rowKey, onSave, onDelete, onCancel }: Props = $props();
  // The row starts from the stored interface and then holds the operator's edits; the owner remounts it when the stored one changes.
  // svelte-ignore state_referenced_locally
  let values = $state(webValuesFrom(saved));
  let errors = $state<FieldError[]>([]);
  let busy = $state(false);
  const changed = $derived(webChanged(values, saved));
  const urlError = $derived(errors.find((error) => error.field === 'url'));
  const labelError = $derived(errors.find((error) => error.field === 'label'));
  const rowErrors = $derived(errors.filter((error) => error.field !== 'url' && error.field !== 'label'));

  async function save(): Promise<void> {
    if (busy || !changed) return;
    errors = validateWeb(values);
    if (errors.length > 0) return;
    busy = true;
    const result = await onSave(webInput(values));
    busy = false;
    if (result.ok) {
      if (saved === undefined) values = webValuesFrom();
    } else {
      errors = fieldErrorsFrom(result.error);
    }
  }
</script>

<form
  class="row"
  novalidate
  onsubmit={(event) => {
    event.preventDefault();
    void save();
  }}
>
  {#each rowErrors as error, index (index)}
    <p class="error" role="alert">{error.message}</p>
  {/each}
  <div class="field">
    <label for="{rowKey}-label">Label</label>
    <input
      id="{rowKey}-label"
      bind:value={values.label}
      placeholder="The host name"
      autocomplete="off"
      aria-invalid={labelError !== undefined}
      aria-describedby={labelError === undefined ? undefined : `${rowKey}-label-error`}
    />
    <small id="{rowKey}-label-error" class="error">{labelError?.message ?? ''}</small>
  </div>
  <div class="field">
    <label for="{rowKey}-url">URL</label>
    <input
      id="{rowKey}-url"
      bind:value={values.url}
      inputmode="url"
      autocomplete="off"
      spellcheck="false"
      aria-invalid={urlError !== undefined}
      aria-describedby={urlError === undefined ? undefined : `${rowKey}-url-error`}
    />
    <small id="{rowKey}-url-error" class="error">{urlError?.message ?? ''}</small>
  </div>
  <div class="field">
    <label for="{rowKey}-open-in">Open in</label>
    <select id="{rowKey}-open-in" bind:value={values.openIn}>
      {#each OPEN_IN_CHOICES as choice (choice.value)}
        <option value={choice.value}>{choice.text}</option>
      {/each}
    </select>
  </div>
  <div class="actions">
    <FormButton type="submit" tone="primary" aria-disabled={busy || !changed} aria-busy={busy}>
      {busy ? 'Working' : saved === undefined ? 'Add web interface' : 'Save'}
    </FormButton>
    {#if onDelete !== undefined}
      <FormButton tone="danger" onclick={onDelete}>Delete</FormButton>
    {/if}
    {#if onCancel !== undefined}
      <FormButton onclick={onCancel}>Cancel</FormButton>
    {/if}
  </div>
</form>

<style>
  .row {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding: var(--space-3);
    background: var(--surface);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
  }

  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  label {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  input,
  select {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: var(--space-2) var(--space-3);
    background: var(--surface);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    color: var(--text);
    font: inherit;
  }

  [aria-invalid='true'] {
    border-color: var(--status-down);
  }

  :focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  small {
    font-size: var(--text-xs);
  }

  .error {
    margin: 0;
    color: var(--status-down);
  }

  small.error:empty {
    display: none;
  }

  .actions {
    display: flex;
    gap: var(--space-2);
  }
</style>
