<script lang="ts">
  import type { FieldError } from '$lib/hub/hub';
  import type { HttpAuthKind, HttpMethod } from '../model/hub-types';
  import { isSecretHeader } from '../model/config-input';
  import { messageFor } from '../model/errors';
  import type { CheckFormValues } from '../model/values';
  import PressButton from './PressButton.svelte';
  import SelectField from './SelectField.svelte';
  import TextField from './TextField.svelte';
  import ToggleField from './ToggleField.svelte';

  interface Props {
    http: CheckFormValues['http'];
    errors: readonly FieldError[];
  }

  let { http = $bindable(), errors }: Props = $props();

  const METHODS: readonly HttpMethod[] = ['GET', 'HEAD', 'POST', 'PUT', 'PATCH', 'DELETE', 'OPTIONS'];
  const methodOptions = METHODS.map((value) => ({ value, label: value }));
  const authOptions: readonly { value: HttpAuthKind; label: string }[] = [
    { value: 'NONE', label: 'None' },
    { value: 'BASIC', label: 'Basic (user name and password)' },
    { value: 'BEARER', label: 'Bearer token' },
  ];
  const keepHint = 'Leave empty to keep the stored value.';
  const secretHint = $derived(http.storedAuth === http.authKind ? keepHint : undefined);
  const error = (field: string) => messageFor(errors, field);
</script>

<TextField label="URL" bind:value={http.url} error={error('url')} />
<SelectField label="Method" bind:value={http.method} options={methodOptions} />
<TextField label="Timeout (ms)" kind="numeric" bind:value={http.timeoutMs} error={error('timeoutMs')} />
<TextField label="Maximum redirects" kind="numeric" bind:value={http.maxRedirects} error={error('maxRedirects')} />
<ToggleField label="Verify the TLS certificate" bind:checked={http.verifyTls} />

<fieldset>
  <legend>Accepted status codes</legend>
  {#each http.acceptedStatus as range, index (index)}
    <div class="row">
      <TextField label={`Range ${String(index + 1)} from`} kind="numeric" bind:value={range.from} />
      <TextField label={`Range ${String(index + 1)} to`} kind="numeric" bind:value={range.to} error={error(`acceptedStatus.${String(index)}`)} />
      <PressButton label={`Remove range ${String(index + 1)}`} onclick={() => http.acceptedStatus.splice(index, 1)} />
    </div>
  {/each}
  {#if error('acceptedStatus') !== undefined}<small class="error" role="alert">{error('acceptedStatus')}</small>{/if}
  <PressButton label="Add range" onclick={() => http.acceptedStatus.push({ from: '200', to: '299' })} />
</fieldset>

<fieldset>
  <legend>Headers</legend>
  {#each http.headers as header, index (index)}
    <div class="row">
      <TextField label={`Header name ${String(index + 1)}`} bind:value={header.name} error={error(`headers.${String(index)}.name`)} />
      <TextField
        label={`Header value ${String(index + 1)}`}
        kind={isSecretHeader(header.name) ? 'password' : 'text'}
        bind:value={header.value}
        error={error(`headers.${String(index)}.value`)}
        hint={header.stored && isSecretHeader(header.name) ? keepHint : undefined}
      />
      <PressButton label={`Remove header ${String(index + 1)}`} onclick={() => http.headers.splice(index, 1)} />
    </div>
  {/each}
  {#if error('headers') !== undefined}<small class="error" role="alert">{error('headers')}</small>{/if}
  <PressButton label="Add header" onclick={() => http.headers.push({ name: '', value: '', stored: false })} />
</fieldset>

<TextField label="Request body" kind="multiline" bind:value={http.body} error={error('body')} />
<TextField label="Keyword the response must contain" bind:value={http.keyword} error={error('keyword')} />
<TextField label="JSON pointer to check" bind:value={http.jsonQuery} error={error('jsonQuery')} />
<TextField label="Expected JSON value" bind:value={http.jsonExpected} error={error('jsonExpected')} />

<SelectField label="Authentication" bind:value={http.authKind} options={authOptions} error={error('auth')} />
{#if http.authKind === 'BASIC'}
  <TextField label="User name" bind:value={http.username} error={error('username')} hint={http.storedAuth === 'BASIC' ? 'Enter the stored user name again; the hub never returns it. If you change the user name, type the password again.' : undefined} />
  <TextField label="Password" kind="password" bind:value={http.password} error={error('password')} hint={secretHint} />
{:else if http.authKind === 'BEARER'}
  <TextField label="Bearer token" kind="password" bind:value={http.bearerToken} error={error('bearerToken')} hint={secretHint} />
{/if}

<style>
  fieldset {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    margin: 0;
    padding: var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
  }

  legend {
    padding: 0 var(--space-1);
    font-size: var(--text-sm);
    color: var(--text-muted);
  }

  .row {
    display: grid;
    grid-template-columns: 1fr 1fr auto;
    gap: var(--space-2);
    align-items: end;
  }

  .error {
    font-size: var(--text-xs);
    color: var(--status-down);
  }
</style>
