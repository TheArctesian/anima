<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import Field from '../../Field.svelte';
  import type { ChannelDraft, Stored } from '../draft';
  import type { DraftField } from '../input';
  import SecretField from '../SecretField.svelte';

  interface Props {
    webhook: ChannelDraft['webhook'];
    /** `null` for a new channel. */
    stored: Stored | null;
    problemOf: (field: DraftField) => string;
  }

  let { webhook = $bindable(), stored, problemOf }: Props = $props();
  const storedNames = $derived(stored?.headerNames ?? []);

  /** Replacing starts from the stored names; every value must be typed again, since the hub never returns one. */
  function replaceChanged(): void {
    webhook.headers = webhook.replaceHeaders ? storedNames.map((name) => ({ name, value: '' })) : [];
  }
</script>

<SecretField label="Webhook URL" bind:value={webhook.url} problem={problemOf('webhook.url')} stored={stored !== null} hint="The list shows only its origin." />
<Field label="Method" problem="">
  {#snippet control(attributes)}
    <select {...attributes} bind:value={webhook.method}>
      <option value="POST">POST</option>
      <option value="PUT">PUT</option>
    </select>
  {/snippet}
</Field>
<fieldset class="headers">
  <legend>Headers</legend>
  {#if stored !== null}
    <p class="stored">{storedNames.length === 0 ? 'No headers stored.' : `Stored: ${storedNames.join(', ')}. Their values are kept sealed.`}</p>
    <label class="check">
      <input type="checkbox" bind:checked={webhook.replaceHeaders} onchange={replaceChanged} />
      Replace the headers
    </label>
  {/if}
  {#if webhook.replaceHeaders}
    {#each webhook.headers as header, index (index)}
      <div class="row">
        <Field label="Header {index + 1} name" problem="">
          {#snippet control(attributes)}
            <input {...attributes} type="text" bind:value={header.name} autocomplete="off" autocapitalize="off" spellcheck="false" />
          {/snippet}
        </Field>
        <SecretField label="Header {index + 1} value" shape="hidden" bind:value={header.value} problem="" />
        <FormButton
          tabindex={0}
          aria-label="Remove header {index + 1}"
          onclick={() => {
            webhook.headers.splice(index, 1);
          }}>Remove</FormButton
        >
      </div>
    {/each}
    <div>
      <FormButton
        tabindex={0}
        onclick={() => {
          webhook.headers.push({ name: '', value: '' });
        }}>Add header</FormButton
      >
    </div>
  {/if}
  <small class="problem">{problemOf('webhook.headers')}</small>
</fieldset>

<style>
  .headers {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-inline-size: 0;
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    padding: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .stored {
    margin: 0;
    font-size: var(--text-sm);
    overflow-wrap: anywhere;
  }

  .check {
    display: flex;
    min-block-size: var(--target-min-desktop);
    align-items: center;
    gap: var(--space-2);
  }

  .check input:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  /* Name and value side by side where there is room, under each other on a phone. */
  .row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(calc(var(--space-16) * 2), 1fr));
    align-items: end;
    gap: var(--space-2);
    padding-block-end: var(--space-2);
    border-block-end: var(--border-thin) solid var(--border-strong);
  }

  .problem {
    min-block-size: calc(var(--text-xs) * var(--leading-normal));
    font-size: var(--text-xs);
    line-height: var(--leading-normal);
  }

  .problem:not(:empty) {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }
</style>
