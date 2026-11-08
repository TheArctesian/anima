<script lang="ts">
  import type { FieldError } from '$lib/hub/hub';
  import { messageFor } from '../model/errors';
  import type { CheckFormValues } from '../model/values';
  import TextField from './TextField.svelte';
  import ToggleField from './ToggleField.svelte';

  interface Props {
    values: CheckFormValues;
    errors: readonly FieldError[];
  }

  let { values = $bindable(), errors }: Props = $props();
  const error = (field: string) => messageFor(errors, field);
</script>

<TextField label="Interval (seconds)" kind="numeric" bind:value={values.intervalSeconds} error={error('intervalSeconds')} />
<TextField label="Retries before Down" kind="numeric" bind:value={values.retries} error={error('retries')} />
<TextField
  label="Retry interval (seconds)"
  kind="numeric"
  bind:value={values.retryIntervalSeconds}
  error={error('retryIntervalSeconds')}
/>
<TextField
  label="Degraded threshold (ms)"
  kind="numeric"
  bind:value={values.degradedThresholdMs}
  error={error('degradedThresholdMs')}
/>
<ToggleField
  label="Invert"
  bind:checked={values.invert}
  hint="A failed test gives the status Up. Use it for a port that must be closed."
/>
