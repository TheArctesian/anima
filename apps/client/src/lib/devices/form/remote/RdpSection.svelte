<script lang="ts">
  import type { FieldError } from '$lib/hub/hub';
  import FormButton from '../../../forms/FormButton.svelte';
  import type { EditedValues } from '../form/editing';
  import Field from '../form/Field.svelte';
  import type { RdpResizeMethod, RdpSecurity } from '$lib/hub/generated/graphql';
  import type { ServerCertificate } from '$lib/sessions/api';
  import { rdpWarnings, validUntil } from './warnings';

  interface Props {
    values: EditedValues;
    errors: readonly FieldError[];
    /** K1 `credentialStored`: the form never receives the secret itself (DEV-011). */
    credentialStored: boolean;
    /** The certificate the hub recorded on the first connection, shown read-only. */
    serverCertificate?: ServerCertificate | null;
    clearing: boolean;
    /** Without it the Remove button is hidden (D-4). */
    onClearCredential?: () => void;
  }

  let { values = $bindable(), errors, credentialStored, serverCertificate = null, clearing, onClearCredential }: Props = $props();

  const SECURITY: readonly { value: RdpSecurity; label: string }[] = [
    { value: 'ANY', label: 'Any (the server chooses)' },
    { value: 'NLA', label: 'Network Level Authentication' },
    { value: 'NLA_EXT', label: 'Network Level Authentication, extended' },
    { value: 'TLS', label: 'TLS' },
    { value: 'VMCONNECT', label: 'Hyper-V virtual machine' },
    { value: 'RDP', label: 'Standard RDP (weak)' },
  ];
  const RESIZE: readonly { value: RdpResizeMethod; label: string }[] = [
    { value: 'DISPLAY_UPDATE', label: 'Display update' },
    { value: 'RECONNECT', label: 'Reconnect' },
  ];
  const warnings = $derived(rdpWarnings(values.rdp));
</script>

<label class="check">
  <input type="checkbox" bind:checked={values.rdp.enabled} />
  RDP enabled
</label>
{#if values.rdp.enabled}
  <Field label="Port" field="rdp.port" {errors}>
    {#snippet control(attributes)}
      <input {...attributes} bind:value={values.rdp.port} inputmode="numeric" autocomplete="off" />
    {/snippet}
  </Field>
  <Field label="User name" field="rdp.username" {errors}>
    {#snippet control(attributes)}
      <input {...attributes} bind:value={values.rdp.username} autocomplete="off" spellcheck="false" />
    {/snippet}
  </Field>
  <Field label="Domain" field="rdp.domain" {errors} hint="Optional.">
    {#snippet control(attributes)}
      <input {...attributes} bind:value={values.rdp.domain} autocomplete="off" spellcheck="false" />
    {/snippet}
  </Field>
  <Field label="Password" field="rdp.password" {errors}>
    {#snippet control(attributes)}
      <input {...attributes} type="password" autocomplete="new-password" bind:value={values.remoteSecrets.rdpPassword} />
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
  <Field label="Security" field="rdp.security" {errors}>
    {#snippet control(attributes)}
      <select {...attributes} bind:value={values.rdp.security}>
        {#each SECURITY as option (option.value)}
          <option value={option.value}>{option.label}</option>
        {/each}
      </select>
    {/snippet}
  </Field>
  <Field label="When the window is resized" field="rdp.resizeMethod" {errors}>
    {#snippet control(attributes)}
      <select {...attributes} bind:value={values.rdp.resizeMethod}>
        {#each RESIZE as option (option.value)}
          <option value={option.value}>{option.label}</option>
        {/each}
      </select>
    {/snippet}
  </Field>
  <label class="check"><input type="checkbox" bind:checked={values.rdp.ignoreCertificate} />Ignore the server certificate</label>
  <label class="check"><input type="checkbox" bind:checked={values.rdp.audioOut} />Play sound from the device</label>
  <label class="check"><input type="checkbox" bind:checked={values.rdp.microphone} />Send the microphone</label>
  <label class="check"><input type="checkbox" bind:checked={values.rdp.sharedDrive} />Share a drive</label>
  {#each warnings as warning (warning)}
    <p class="warning" role="note">{warning}</p>
  {/each}
  {#if serverCertificate !== null}
    <dl class="certificate" aria-label="Server certificate">
      <dt>Fingerprint (SHA-256)</dt>
      <dd>{serverCertificate.fingerprintSha256}</dd>
      {#if serverCertificate.subject}
        <dt>Subject</dt>
        <dd>{serverCertificate.subject}</dd>
      {/if}
      {#if serverCertificate.notAfter}
        <dt>Valid until</dt>
        <dd>{validUntil(serverCertificate.notAfter)}</dd>
      {/if}
    </dl>
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

  .certificate {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--space-1) var(--space-3);
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  dt {
    color: var(--text-muted);
  }

  dd {
    margin: 0;
    color: var(--text);
    font-family: var(--font-mono);
    overflow-wrap: anywhere;
  }
</style>
