<script lang="ts">
  import Menu from '$lib/devices/toolbar/Menu.svelte';
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import type { AgentSource } from '$lib/hub/generated/graphql';
  import type { UsageDevice } from './api';
  import { DAY_CHOICES, GROUP_CHOICES, SOURCE_CHOICES, withGroup, type Days, type UsageQuery } from './query';

  interface Props {
    query: UsageQuery;
    /** Every device whose daemon sends counts, so the choice stays whole when one device is chosen. */
    devices: readonly UsageDevice[];
    onChange: (query: UsageQuery) => void;
  }

  const { query, devices, onChange }: Props = $props();
  const ALL = '';
  const DAYS = DAY_CHOICES.map((days) => ({ value: String(days), label: `${String(days)} days` }));
  const deviceOptions = $derived([
    { value: ALL, label: 'All devices' },
    ...devices.map(({ device }) => ({ value: device.id, label: device.name })),
  ]);
  const deviceShown = $derived(
    query.device === null ? 'All devices' : (deviceOptions.find((option) => option.value === query.device)?.label ?? 'Unknown device'),
  );
  const sourceOptions = [{ value: ALL, label: 'All programs' }, ...SOURCE_CHOICES];
  const sourceShown = $derived(sourceOptions.find((option) => option.value === (query.source ?? ALL))?.label ?? 'All programs');
</script>

<!-- One row of choices; each is in the address of the page, so Back and a copied link restore them. -->
<div class="filters" role="group" aria-label="Usage filters">
  <Segmented
    label="Days"
    size="sm"
    options={DAYS}
    value={String(query.days)}
    onchange={(value) => {
      onChange({ ...query, days: Number(value) as Days });
    }}
  />
  <Menu
    label="Group by"
    icon="carbon:list"
    kind="checkbox"
    options={GROUP_CHOICES}
    chosen={query.group}
    onChoose={(value) => {
      onChange({ ...query, group: withGroup(query.group, value as UsageQuery['group'][number]) });
    }}
  />
  <Menu
    label="Device"
    shown={deviceShown}
    icon="carbon:bare-metal-server"
    kind="radio"
    options={deviceOptions}
    chosen={[query.device ?? ALL]}
    onChoose={(value) => {
      onChange({ ...query, device: value === ALL ? null : value });
    }}
  />
  <Menu
    label="Program"
    shown={sourceShown}
    icon="carbon:terminal"
    kind="radio"
    options={sourceOptions}
    chosen={[query.source ?? ALL]}
    onChoose={(value) => {
      onChange({ ...query, source: value === ALL ? null : (value as AgentSource) });
    }}
  />
</div>

<style>
  .filters {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-3);
    min-inline-size: 0;
    padding-block-end: var(--space-3);
    border-block-end: var(--border-thin) solid var(--border-subtle);
  }
</style>
