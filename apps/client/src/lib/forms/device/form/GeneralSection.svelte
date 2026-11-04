<script lang="ts">
  import type { FieldError } from '$lib/hub/error';
  import type { OperatingSystem, SiteOption } from '../hub-types';
  import type { EditedValues } from './editing';
  import Field from './Field.svelte';
  import TagsField from './TagsField.svelte';

  interface Props {
    values: EditedValues;
    errors: readonly FieldError[];
    /** Offered in create mode when there is more than one (D-6). */
    sites: readonly SiteOption[];
    siteId: string;
    onHostBlur: () => void;
  }

  let { values = $bindable(), errors, sites, siteId = $bindable(), onHostBlur }: Props = $props();

  const SYSTEMS: readonly { value: OperatingSystem; label: string }[] = [
    { value: 'LINUX', label: 'Linux' },
    { value: 'MACOS', label: 'macOS' },
    { value: 'WINDOWS', label: 'Windows' },
    { value: 'OTHER', label: 'Other' },
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
<TagsField bind:tags={values.tags} />
