<script lang="ts">
  import type { FieldError } from '$lib/hub/error';
  import TextField from '../fields/TextField.svelte';
  import { messageFor } from '../model/errors';
  import type { CheckFormValues } from '../model/values';

  interface Props {
    /** DOCKER, SYSTEMD or TAILSCALE_PING; SNMP has its own component. */
    values: CheckFormValues;
    errors: readonly FieldError[];
  }

  let { values = $bindable(), errors }: Props = $props();
  const error = (field: string) => messageFor(errors, field);
</script>

{#if values.type === 'DOCKER'}
  <TextField label="Container name" bind:value={values.docker.container} error={error('container')} />
  <TextField label="Timeout (ms)" kind="numeric" bind:value={values.docker.timeoutMs} error={error('timeoutMs')} />
{:else if values.type === 'SYSTEMD'}
  <TextField
    label="Unit name"
    bind:value={values.systemd.unit}
    error={error('unit')}
    hint="For example nginx.service."
  />
  <TextField label="Timeout (ms)" kind="numeric" bind:value={values.systemd.timeoutMs} error={error('timeoutMs')} />
{:else if values.type === 'TAILSCALE_PING'}
  <TextField
    label="Target"
    bind:value={values.tailscalePing.target}
    error={error('target')}
    hint="Leave empty to use the device host."
  />
  <TextField label="Timeout (ms)" kind="numeric" bind:value={values.tailscalePing.timeoutMs} error={error('timeoutMs')} />
{/if}
