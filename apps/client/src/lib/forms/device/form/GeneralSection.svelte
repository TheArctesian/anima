<script lang="ts">
  import type { FieldError } from '$lib/hub/error';
  import type { MetricsSource, OperatingSystem } from '$lib/hub/generated/graphql';
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

  const SYSTEMS: readonly { value: OperatingSystem; label: string }[] = [
    { value: 'LINUX', label: 'Linux' },
    { value: 'MACOS', label: 'macOS' },
    { value: 'WINDOWS', label: 'Windows' },
    { value: 'OTHER', label: 'Other' },
  ];
  const METRICS: readonly { value: MetricsSource; label: string }[] = [
    { value: 'NONE', label: 'None' },
    { value: 'SSH', label: 'SSH' },
  ];
  const KINDS = ['server', 'router', 'switch', 'workstation', 'laptop', 'printer'] as const;
</script>

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
  <Field label="Site" field="siteId" {errors}>
    {#snippet control(attributes)}
      <select {...attributes} bind:value={siteId}>
        {#each sites as site (site.id)}
          <option value={site.id}>{site.name}</option>
        {/each}
      </select>
    {/snippet}
  </Field>
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
    <select {...attributes} bind:value={values.os}>
      {#each SYSTEMS as system (system.value)}
        <option value={system.value}>{system.label}</option>
      {/each}
    </select>
  {/snippet}
</Field>
<Field label="MAC address" field="macAddress" {errors} hint="Optional. Lets Anima wake the device. For example aa:bb:cc:dd:ee:ff.">
  {#snippet control(attributes)}
    <input {...attributes} bind:value={values.macAddress} autocomplete="off" spellcheck="false" />
  {/snippet}
</Field>
<Field label="Metrics source" field="metricsSource" {errors}>
  {#snippet control(attributes)}
    <select {...attributes} bind:value={values.metricsSource}>
      {#each METRICS as source (source.value)}
        <option value={source.value}>{source.label}</option>
      {/each}
    </select>
  {/snippet}
</Field>
<TagsField bind:tags={values.tags} />
