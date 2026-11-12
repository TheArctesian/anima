<script lang="ts">
  import type { FieldError } from '$lib/hub/hub';
  import type { DnsRecordType, IpVersion } from '../model/hub-types';
  import { messageFor } from '../model/errors';
  import type { CheckFormValues } from '../model/values';
  import SelectField from './SelectField.svelte';
  import TextField from './TextField.svelte';

  interface Props {
    /** ICMP, TCP, TLS, DNS or PUSH; HTTP and GROUP have their own components. */
    values: CheckFormValues;
    errors: readonly FieldError[];
  }

  let { values = $bindable(), errors }: Props = $props();

  const ipVersions: readonly { value: IpVersion; label: string }[] = [
    { value: 'V4', label: 'IPv4' },
    { value: 'V6', label: 'IPv6' },
  ];
  const RECORD_TYPES: readonly DnsRecordType[] = ['A', 'AAAA', 'CNAME', 'MX', 'NS', 'TXT', 'SRV', 'PTR'];
  const recordTypes = RECORD_TYPES.map((value) => ({ value, label: value }));
  const error = (field: string) => messageFor(errors, field);
</script>

{#if values.type === 'ICMP'}
  <TextField label="Timeout (ms)" kind="numeric" bind:value={values.icmp.timeoutMs} error={error('timeoutMs')} />
  <SelectField label="IP version" bind:value={values.icmp.ipVersion} options={ipVersions} />
{:else if values.type === 'TCP'}
  <TextField label="Port" kind="numeric" bind:value={values.tcp.port} error={error('port')} />
  <TextField label="Timeout (ms)" kind="numeric" bind:value={values.tcp.timeoutMs} error={error('timeoutMs')} />
{:else if values.type === 'TLS'}
  <TextField label="Port" kind="numeric" bind:value={values.tls.port} error={error('port')} />
  <TextField
    label="Server name"
    bind:value={values.tls.serverName}
    error={error('serverName')}
    hint="Leave empty to use the device host."
  />
  <TextField label="Timeout (ms)" kind="numeric" bind:value={values.tls.timeoutMs} error={error('timeoutMs')} />
{:else if values.type === 'DNS'}
  <TextField label="Name to look up" bind:value={values.dns.name} error={error('dnsName')} />
  <SelectField label="Record type" bind:value={values.dns.recordType} options={recordTypes} />
  <TextField
    label="Resolver"
    bind:value={values.dns.resolver}
    error={error('resolver')}
    hint="An IP address. Leave empty to use the hub's resolver."
  />
  <TextField label="Expected answer" bind:value={values.dns.expected} error={error('expected')} />
{:else if values.type === 'PUSH'}
  {#if values.push.path === ''}
    <p class="note">The hub creates the push address when you save. The device calls it at least once per interval.</p>
  {:else}
    <TextField label="Push address (on the hub)" bind:value={values.push.path} disabled />
  {/if}
{/if}

<style>
  .note {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }
</style>
