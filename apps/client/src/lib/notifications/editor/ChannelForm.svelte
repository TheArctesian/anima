<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import { modalFooter } from '$lib/forms/modal/footer';
  import Field from '../Field.svelte';
  import TemplateFields from '../template/TemplateFields.svelte';
  import AppriseFields from './kinds/AppriseFields.svelte';
  import { KINDS, type ChannelDraft, type Stored } from './draft';
  import EmailFields from './kinds/EmailFields.svelte';
  import type { DraftField } from './input';
  import NtfyFields from './kinds/NtfyFields.svelte';
  import WebhookFields from './kinds/WebhookFields.svelte';

  // The editor's fields, laid out; the editor around it owns the values' life, the check and the save.
  interface Props {
    draft: ChannelDraft;
    /** `null` for a new channel, which alone may choose its kind. */
    stored: Stored | null;
    problemOf: (field: DraftField) => string;
    /** A fixed sentence for a refused save, or empty. */
    failure: string;
    busy: boolean;
    placeholders: readonly string[];
    onsubmit: (event: SubmitEvent) => void;
    onCancel: () => void;
  }

  let { draft = $bindable(), stored, problemOf, failure, busy, placeholders, onsubmit, onCancel }: Props = $props();
  const ids = $props.id();
  const kindLabel = $derived(KINDS.find((kind) => kind.id === draft.kind)?.label ?? '');
  const templateProblems = $derived({ title: problemOf('template.title'), body: problemOf('template.body') });

  // Inside the modal the actions go to its footer; the Save button is tied to the form by its ID.
  const footer = modalFooter();
  $effect.pre(() => footer?.show(actions));
</script>

<form id="{ids}-form" {onsubmit} novalidate>
  <fieldset class="fields" disabled={busy}>
    <Field label="Name" problem={problemOf('name')}>
      {#snippet control(attributes)}
        <input {...attributes} type="text" bind:value={draft.name} autocomplete="off" spellcheck="false" />
      {/snippet}
    </Field>
    {#if stored === null}
      <fieldset class="choice">
        <legend>Kind</legend>
        {#each KINDS as kind (kind.id)}
          <label><input type="radio" name="{ids}-kind" value={kind.id} bind:group={draft.kind} /> {kind.label}</label>
        {/each}
      </fieldset>
    {/if}
    <label class="check">
      <input type="checkbox" bind:checked={draft.onByDefault} aria-describedby="{ids}-default-hint" />
      On by default
    </label>
    <small id="{ids}-default-hint">New checks start with this channel on.</small>
    <fieldset class="group">
      <legend>{kindLabel}</legend>
      {#if draft.kind === 'WEBHOOK'}
        <WebhookFields bind:webhook={draft.webhook} {stored} {problemOf} />
      {:else if draft.kind === 'NTFY'}
        <NtfyFields bind:ntfy={draft.ntfy} {stored} {problemOf} />
      {:else if draft.kind === 'EMAIL'}
        <EmailFields bind:email={draft.email} {stored} {problemOf} />
      {:else}
        <AppriseFields bind:apprise={draft.apprise} {stored} {problemOf} />
      {/if}
    </fieldset>
    <fieldset class="group">
      <legend>Message</legend>
      <label class="check">
        <input type="checkbox" bind:checked={draft.ownTemplate} aria-describedby="{ids}-template-hint" />
        Own template
      </label>
      <small id="{ids}-template-hint">Off: the default template below in Notifications.</small>
      <TemplateFields bind:template={draft.template} problems={templateProblems} {placeholders} disabled={!draft.ownTemplate} />
    </fieldset>
  </fieldset>
</form>

{#snippet actions()}
  <!-- One line kept for the refusal, so the buttons never move (MOT-007). -->
  <p class="failure" role="alert">{failure}</p>
  <div class="actions">
    <FormButton tabindex={0} onclick={onCancel} aria-disabled={busy}>Cancel</FormButton>
    <!-- The label stays while the hub works, so neither button moves (MOT-007); aria-busy says it is busy. -->
    <FormButton tabindex={0} type="submit" form="{ids}-form" tone="primary" aria-disabled={busy} aria-busy={busy}>Save</FormButton>
  </div>
{/snippet}

<style>
  .fields,
  .group,
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

  .group {
    padding-block-start: var(--space-3);
    border-block-start: var(--border-thin) solid var(--border-strong);
  }

  legend {
    padding: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .group > legend {
    color: var(--text);
    font-size: var(--text-md);
    font-weight: var(--weight-bold);
  }

  .choice {
    flex-flow: row wrap;
    column-gap: var(--space-4);
  }

  /* The whole row is the target, at least the desktop target size (UI-013). */
  .choice label,
  .check {
    display: flex;
    min-block-size: var(--target-min-desktop);
    align-items: center;
    gap: var(--space-2);
  }

  input:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  small {
    margin-block-start: calc(-1 * var(--space-2));
    color: var(--text-secondary);
    font-size: var(--text-xs);
  }

  .failure {
    min-block-size: calc(var(--text-sm) * var(--leading-normal));
    margin: 0 0 var(--space-2);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  .failure:not(:empty) {
    padding-inline-start: var(--space-2);
    border-inline-start: var(--border-focus) solid var(--status-down);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
</style>
