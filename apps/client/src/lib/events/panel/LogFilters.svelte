<script lang="ts">
  import Menu from '$lib/devices/toolbar/Menu.svelte';
  import type { LogFilter } from '../log';
  import { switched, withDevice, withSince } from './filters';
  import { SEVERITY_CHOICES, SINCE_CHOICES, SOURCE_CHOICES } from './view';

  interface Props {
    filter: LogFilter;
    /** The devices the page already has. */
    devices: readonly { id: string; name: string }[];
    onChange: (filter: LogFilter) => void;
  }

  const { filter, devices, onChange }: Props = $props();
</script>

<div class="filters" role="group" aria-label="Log filters">
  <label class="select">
    <span>Time</span>
    <select
      value={filter.since}
      onchange={(event) => {
        onChange(withSince(filter, event.currentTarget.value));
      }}
    >
      {#each SINCE_CHOICES as choice (choice.value)}
        <option value={choice.value}>{choice.label}</option>
      {/each}
    </select>
  </label>
  <Menu
    label="Severity"
    icon="nf-md-alert"
    kind="checkbox"
    options={SEVERITY_CHOICES}
    chosen={filter.severities}
    onChoose={(value) => {
      onChange({ ...filter, severities: switched(SEVERITY_CHOICES, filter.severities, value) });
    }}
  />
  <Menu
    label="Source"
    icon="nf-md-filter_variant"
    kind="checkbox"
    options={SOURCE_CHOICES}
    chosen={filter.sources}
    onChoose={(value) => {
      onChange({ ...filter, sources: switched(SOURCE_CHOICES, filter.sources, value) });
    }}
  />
  <label class="select">
    <span>Device</span>
    <select
      value={filter.deviceId ?? ''}
      onchange={(event) => {
        onChange(withDevice(filter, event.currentTarget.value));
      }}
    >
      <option value="">All devices</option>
      {#each devices as device (device.id)}
        <option value={device.id}>{device.name}</option>
      {/each}
    </select>
  </label>
</div>

<style>
  .filters {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
  }

  .select {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--text-muted);
  }

  select {
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-2);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  select:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }
</style>
