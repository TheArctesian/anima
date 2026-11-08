<script lang="ts">
  import type { FieldError } from '$lib/hub/hub';
  import FormButton from '../../../forms/FormButton.svelte';
  import type { EditedValues } from '../form/editing';
  import Field from '../form/Field.svelte';

  interface Props {
    values: EditedValues;
    errors: readonly FieldError[];
    /** K1 `credentialStored`: the form never receives the secret itself (DEV-011). */
    credentialStored: boolean;
    /** The hub's UNENCRYPTED_VNC warning for the saved device, as a sentence; null when it has none (RA-020). */
    warning?: string | null;
    clearing: boolean;
    /** Without it the Remove button is hidden (D-4). */
    onClearCredential?: () => void;
  }

  let { values = $bindable(), errors, credentialStored, warning = null, clearing, onClearCredential }: Props = $props();
</script>

<label class="check">
  <input type="checkbox" bind:checked={values.vnc.enabled} />
  VNC enabled
</label>
{#if values.vnc.enabled}
  <Field label="Port" field="vnc.port" {errors}>
    {#snippet control(attributes)}
      <input {...attributes} bind:value={values.vnc.port} inputmode="numeric" autocomplete="off" />
    {/snippet}
  </Field>
  <Field label="User name" field="vnc.username" {errors} hint="Optional. macOS Screen Sharing needs one.">
    {#snippet control(attributes)}
      <input {...attributes} bind:value={values.vnc.username} autocomplete="off" spellcheck="false" />
    {/snippet}
  </Field>
  <Field label="Password" field="vnc.password" {errors}>
    {#snippet control(attributes)}
      <input {...attributes} type="password" autocomplete="new-password" bind:value={values.remoteSecrets.vncPassword} />
    {/snippet}
  </Field>
  {#if credentialStored}
    <p class="stored">
      Credential stored. Leave the password empty to keep it.
      {#if onClearCredential !== undefined}
        <FormButton onclick={onClearCredential} aria-disabled={clearing}>Remove credential</FormButton>
      {/if}
    </p>
  {/if}
  {#if warning !== null}
    <p class="warning" role="note">{warning}</p>
  {/if}
{/if}

<style>
  .check {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .stored {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
    margin: 0;
    color: var(--status-up);
  }

  .warning {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    background: var(--status-warning-tint);
    border-radius: var(--radius-md);
    color: var(--status-warning);
  }
</style>
