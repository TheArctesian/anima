<script lang="ts">
  import type { FieldError } from '$lib/hub/hub';
  import type { SettingKey } from '../../defaults/settings';
  import { messageFor } from '../model/errors';
  import type { CheckFormValues } from '../model/values';
  import { takesRetries } from '../probes/domain-expiry';
  import TextField from './TextField.svelte';
  import ToggleField from './ToggleField.svelte';

  interface Props {
    values: CheckFormValues;
    errors: readonly FieldError[];
    /** Called when the operator types in one of the four settings. */
    ontouch?: (setting: SettingKey) => void;
  }

  let { values = $bindable(), errors, ontouch }: Props = $props();
  const error = (field: string) => messageFor(errors, field);
</script>

<TextField
  label="Interval (seconds)"
  kind="numeric"
  bind:value={values.intervalSeconds}
  error={error('intervalSeconds')}
  oninput={() => ontouch?.('intervalSeconds')}
/>
{#if takesRetries(values.type)}
  <TextField
    label="Retries before Down"
    kind="numeric"
    bind:value={values.retries}
    error={error('retries')}
    oninput={() => ontouch?.('retries')}
  />
  <TextField
    label="Retry interval (seconds)"
    kind="numeric"
    bind:value={values.retryIntervalSeconds}
    error={error('retryIntervalSeconds')}
    oninput={() => ontouch?.('retryIntervalSeconds')}
  />
{/if}
<TextField
  label="Degraded threshold (ms)"
  kind="numeric"
  bind:value={values.degradedThresholdMs}
  error={error('degradedThresholdMs')}
  oninput={() => ontouch?.('degradedThresholdMs')}
/>
<ToggleField
  label="Invert"
  bind:checked={values.invert}
  hint="A failed test gives the status Up. Use it for a port that must be closed."
/>
