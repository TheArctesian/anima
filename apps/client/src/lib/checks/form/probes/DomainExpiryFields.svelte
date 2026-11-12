<script lang="ts">
  import type { FieldError } from '$lib/hub/hub';
  import TextField from '../fields/TextField.svelte';
  import { messageFor } from '../model/errors';
  import { daysLeftText, type DomainExpiryValues } from './domain-expiry';

  interface Props {
    domainExpiry: DomainExpiryValues;
    errors: readonly FieldError[];
  }

  let { domainExpiry = $bindable(), errors }: Props = $props();
  const error = (field: string) => messageFor(errors, field);
</script>

<TextField label="Domain" bind:value={domainExpiry.domain} error={error('domain')} hint="For example example.net." />
<TextField label="Warn days" kind="numeric" bind:value={domainExpiry.warnDays} error={error('warnDays')} hint="Degraded when fewer days are left." />
<TextField label="Timeout (ms)" kind="numeric" bind:value={domainExpiry.timeoutMs} error={error('timeoutMs')} />
<!-- The line keeps its height whether or not the days are known (MOT-007). -->
<p class="days-left">{daysLeftText(domainExpiry.daysLeft)}</p>

<style>
  .days-left {
    margin: 0;
    min-block-size: calc(var(--text-sm) * 1.5);
    font-size: var(--text-sm);
    color: var(--text-secondary);
  }
</style>
