<script lang="ts">
  import type { FieldError } from '$lib/hub/hub';
  import type { MetricsSource, OperatingSystem } from '$lib/hub/generated/graphql';
  import { osLabel } from '../../os/os';
  import OsIcon from '../../os/OsIcon.svelte';
  import type { NewSite } from '../site';
  import type { SiteOption } from '../values';
  import type { EditedValues } from './editing';
  import Field from './Field.svelte';
  import TagsField from './TagsField.svelte';

  interface Props {
    values: EditedValues;
    errors: readonly FieldError[];
    /** Offered in create mode when there is more than one (D-6). */
    sites: readonly SiteOption[];
    siteId: string;
    /** The site created with the device when the hub has none yet; null otherwise. */
    newSite: NewSite | null;
    onHostBlur: () => void;
  }

  let { values = $bindable(), errors, sites, siteId = $bindable(), newSite = $bindable(), onHostBlur }: Props = $props();

  const SYSTEMS: readonly OperatingSystem[] = ['LINUX', 'MACOS', 'WINDOWS', 'OTHER'];
  const METRICS: readonly { value: MetricsSource; label: string }[] = [
    { value: 'NONE', label: 'None' },
    { value: 'SSH', label: 'SSH' },
  ];
  /** D-7: only enrolment sets AGENT, so it is shown as the saved state and never offered as a choice. */
  const agentSet = $derived(values.metricsSource === 'AGENT');
  const KINDS = ['server', 'router', 'switch', 'workstation', 'laptop', 'printer'] as const;
</script>

<!-- Short fields in pairs: Name and Host, Kind and Operating system, MAC address and Metrics source. -->
<div class="general">
  <div class="grid">
    <Field label="Name" field="name" {errors}>
      {#snippet control(attributes)}
        <input {...attributes} bind:value={values.name} autocomplete="off" />
      {/snippet}
    </Field>
    <Field label="Host" field="host" {errors} hint="A host name or an IP address.">
      {#snippet control(attributes)}
        <input {...attributes} bind:value={values.host} onblur={onHostBlur} autocomplete="off" spellcheck="false" />
      {/snippet}
    </Field>
    {#if sites.length > 1}
      <div class="wide">
        <Field label="Site" field="siteId" {errors}>
          {#snippet control(attributes)}
            <select {...attributes} bind:value={siteId}>
              {#each sites as site (site.id)}
                <option value={site.id}>{site.name}</option>
              {/each}
            </select>
          {/snippet}
        </Field>
      </div>
    {/if}
    {#if newSite !== null}
      <Field label="Site" field="site.name" {errors} hint="No site exists yet, so Anima creates this one with the device.">
        {#snippet control(attributes)}
          <input {...attributes} bind:value={newSite.name} autocomplete="off" />
        {/snippet}
      </Field>
      <Field label="Site subnet" field="site.subnet" {errors} hint="A VPN address belongs in 100.64.0.0/10.">
        {#snippet control(attributes)}
          <input {...attributes} bind:value={newSite.subnet} autocomplete="off" spellcheck="false" />
        {/snippet}
      </Field>
    {/if}
    <Field label="Kind" field="kind" {errors}>
      {#snippet control(attributes)}
        <input {...attributes} bind:value={values.kind} list="device-kind-options" autocomplete="off" />
        <datalist id="device-kind-options">
          {#each KINDS as kind (kind)}
            <option value={kind}></option>
          {/each}
        </datalist>
      {/snippet}
    </Field>
    <Field label="Operating system" field="os" {errors}>
      {#snippet control(attributes)}
        <!-- A select cannot show a logo, so the chosen system's logo sits beside it; the select names it (UI-003). -->
        <span class="system">
          <OsIcon os={values.os} decorative />
          <select {...attributes} bind:value={values.os}>
            {#each SYSTEMS as system (system)}
              <option value={system}>{osLabel(system)}</option>
            {/each}
          </select>
        </span>
      {/snippet}
    </Field>
    <Field label="MAC address" field="macAddress" {errors} hint="Optional. Lets Anima wake the device. For example aa:bb:cc:dd:ee:ff.">
      {#snippet control(attributes)}
        <input {...attributes} bind:value={values.macAddress} autocomplete="off" spellcheck="false" />
      {/snippet}
    </Field>
    <Field label="Metrics source" field="metricsSource" {errors} hint={agentSet ? "Set by the agent. Revoke it, or pick None or SSH, to change." : undefined}>
      {#snippet control(attributes)}
        <select {...attributes} bind:value={values.metricsSource}>
          {#if agentSet}<option value="AGENT" disabled>Agent</option>{/if}
          {#each METRICS as source (source.value)}
            <option value={source.value}>{source.label}</option>
          {/each}
        </select>
      {/snippet}
    </Field>
    <div class="wide"><TagsField bind:tags={values.tags} /></div>
  </div>
</div>

<style>
  .general {
    container: general / inline-size;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    align-items: start;
    gap: var(--space-4);
  }

  .wide {
    grid-column: 1 / -1;
  }

  .system {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-inline-size: 0;
  }

  /* One column where two would be too narrow for a host name. */
  @container general (inline-size < 30rem) {
    .grid {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
