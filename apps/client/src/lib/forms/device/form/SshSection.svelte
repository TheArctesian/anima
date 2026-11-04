<script lang="ts">
  import type { FieldError } from '$lib/hub/error';
  import FormButton from '../../FormButton.svelte';
  import type { EditedValues } from './editing';
  import Field from './Field.svelte';

  interface Props {
    values: EditedValues;
    errors: readonly FieldError[];
    /** K1 `credentialStored`: the form never receives the secret itself (D-4). */
    credentialStored: boolean;
    clearing: boolean;
    onClearCredential: () => void;
  }

  let { values = $bindable(), errors, credentialStored, clearing, onClearCredential }: Props = $props();
</script>

<label class="check">
  <input type="checkbox" bind:checked={values.ssh.enabled} />
  SSH enabled
</label>
{#if values.ssh.enabled}
  <Field label="Port" field="ssh.port" {errors}>
    {#snippet control(attributes)}
      <input {...attributes} bind:value={values.ssh.port} inputmode="numeric" autocomplete="off" />
    {/snippet}
  </Field>
  <Field label="User name" field="ssh.username" {errors}>
    {#snippet control(attributes)}
      <input {...attributes} bind:value={values.ssh.username} autocomplete="off" spellcheck="false" />
    {/snippet}
  </Field>
  <fieldset>
    <legend>Authentication</legend>
    <label class="check"><input type="radio" name="device-ssh-auth" value="PASSWORD" bind:group={values.ssh.auth} />Password</label>
    <label class="check">
      <input type="radio" name="device-ssh-auth" value="PRIVATE_KEY" bind:group={values.ssh.auth} />Private key
    </label>
  </fieldset>
  {#if values.ssh.auth === 'PASSWORD'}
    <Field label="Password" field="credential.password" {errors}>
      {#snippet control(attributes)}
        <input {...attributes} type="password" autocomplete="off" bind:value={values.credential.password} />
      {/snippet}
    </Field>
  {:else}
    <Field label="Private key" field="credential.privateKey.key" {errors}>
      {#snippet control(attributes)}
        <textarea {...attributes} rows="6" spellcheck="false" autocomplete="off" bind:value={values.credential.privateKey}></textarea>
      {/snippet}
    </Field>
    <Field label="Key passphrase" field="credential.passphrase" {errors}>
      {#snippet control(attributes)}
        <input {...attributes} type="password" autocomplete="off" bind:value={values.credential.passphrase} />
      {/snippet}
    </Field>
  {/if}
  {#if credentialStored}
    <p class="stored">
      Credential stored. Leave the fields empty to keep it.
      <FormButton onclick={onClearCredential} disabled={clearing}>Remove credential</FormButton>
    </p>
  {/if}
{/if}

<style>
  .check {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  fieldset {
    display: flex;
    gap: var(--space-4);
    margin: 0;
    padding: 0;
    border: 0;
  }

  legend {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .stored {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
    margin: 0;
    color: var(--status-up);
  }
</style>
