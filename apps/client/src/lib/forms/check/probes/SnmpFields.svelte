<script lang="ts">
  import type { FieldError } from '$lib/hub/error';
  import type { SnmpVersion } from '$lib/forms/hub-pending';
  import SelectField from '../fields/SelectField.svelte';
  import TextField from '../fields/TextField.svelte';
  import { messageFor } from '../model/errors';
  import type { ProbeValues } from './values';

  interface Props {
    snmp: ProbeValues['snmp'];
    errors: readonly FieldError[];
  }

  let { snmp = $bindable(), errors }: Props = $props();

  const versions: readonly { value: SnmpVersion; label: string }[] = [
    { value: 'V2C', label: 'v2c' },
    { value: 'V3', label: 'v3' },
  ];
  const error = (field: string) => messageFor(errors, field);
  /** SEC-005: the hub never returns a secret; an empty box keeps the stored one while the version is unchanged. */
  const keepHint = $derived(
    snmp.storedVersion === null
      ? undefined
      : snmp.storedVersion === snmp.version
        ? 'Leave empty to keep the stored value.'
        : 'Enter the secrets for the new version.',
  );
</script>

<TextField label="Port" kind="numeric" bind:value={snmp.port} error={error('port')} />
<SelectField label="Version" bind:value={snmp.version} options={versions} />
<TextField label="OID" bind:value={snmp.oid} error={error('oid')} hint="Dotted numbers, such as 1.3.6.1.2.1.1.3.0." />
<TextField
  label="Expected value"
  bind:value={snmp.expected}
  error={error('expected')}
  hint="Leave empty to be Up when the OID answers."
/>
{#if snmp.version === 'V2C'}
  <TextField label="Community" kind="password" bind:value={snmp.community} error={error('community')} hint={keepHint} />
{:else}
  <TextField label="User name" bind:value={snmp.username} error={error('username')} hint="authPriv, SHA-256 and AES-128." />
  <TextField label="Authentication password" kind="password" bind:value={snmp.authPassword} error={error('authPassword')} hint={keepHint} />
  <TextField label="Privacy password" kind="password" bind:value={snmp.privacyPassword} error={error('privacyPassword')} hint={keepHint} />
{/if}
<TextField label="Timeout (ms)" kind="numeric" bind:value={snmp.timeoutMs} error={error('timeoutMs')} />
