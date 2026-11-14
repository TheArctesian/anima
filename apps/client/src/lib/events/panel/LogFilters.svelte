<script lang="ts">
  import Menu from '$lib/devices/toolbar/Menu.svelte';
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import type { Severity } from '$lib/hub/generated/graphql';
  import type { LogFilter, Since } from '../log';
  import { SEVERITY_CHOICES, SINCE_CHOICES, SOURCE_CHOICES, switched, withDevice, withSince } from './view';

  interface Props {
    filter: LogFilter;
    /** The devices the page already has. */
    devices: readonly { id: string; name: string }[];
    /** How many rows are in the list, said at the end of the row. */
    shown: number | null;
    onChange: (filter: LogFilter) => void;
  }

  const { filter, devices, shown, onChange }: Props = $props();
  /** No severity chosen means every severity, so every chip shows on. */
  const severityOn = (value: Severity) => filter.severities.length === 0 || filter.severities.includes(value);

  /** A chip goes off or on; the last one on stays on, and all on is the same as none chosen (LOG-009). */
  function flipSeverity(value: Severity): void {
    const on = SEVERITY_CHOICES.map((choice) => choice.value).filter((each) => severityOn(each));
    const next = switched(SEVERITY_CHOICES, on, value);
    if (next.length === 0) return;
    onChange({ ...filter, severities: next.length === SEVERITY_CHOICES.length ? [] : next });
  }

  const deviceOptions = $derived([{ value: '', label: 'All devices' }, ...devices.map((device) => ({ value: device.id, label: device.name }))]);
  const deviceShown = $derived(deviceOptions.find((option) => option.value === (filter.deviceId ?? ''))?.label ?? 'All devices');
</script>

<!-- One row of filters (UR-7, LOG-009): the window as a segmented choice, severity as chips with their colour and word,
     source and device as menus that say what they show. -->
<div class="filters" role="group" aria-label="Log filters">
  <Segmented
    label="Time"
    size="sm"
    options={SINCE_CHOICES}
    value={filter.since}
    onchange={(value: Since) => {
      onChange(withSince(filter, value));
    }}
  />
  <div class="chips" role="group" aria-label="Severity">
    {#each SEVERITY_CHOICES as choice (choice.value)}
      <button
        type="button"
        class="chip {choice.value.toLowerCase()}"
        tabindex="0"
        aria-pressed={severityOn(choice.value)}
        onclick={() => {
          flipSeverity(choice.value);
        }}
      >
        <span class="dot" aria-hidden="true"></span>{choice.label}
      </button>
    {/each}
  </div>
  <Menu
    label="Source"
    icon="carbon:filter"
    kind="checkbox"
    options={SOURCE_CHOICES}
    chosen={filter.sources}
    onChoose={(value) => {
      onChange({ ...filter, sources: switched(SOURCE_CHOICES, filter.sources, value) });
    }}
  />
  <Menu
    label="Device"
    shown={deviceShown}
    icon="carbon:bare-metal-server"
    kind="radio"
    options={deviceOptions}
    chosen={[filter.deviceId ?? '']}
    onChoose={(value) => {
      onChange(withDevice(filter, value));
    }}
  />
  {#if shown !== null}<span class="shown">{shown === 1 ? '1 event' : `${String(shown)} events`}</span>{/if}
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

  .chips {
    display: flex;
    gap: var(--space-1);
  }

  .chip {
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    block-size: var(--control-sm);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-pill);
    background: none;
    color: var(--text-tertiary);
    font: inherit;
    font-size: var(--text-sm);
    transition: background-color var(--duration-fast) var(--ease-enter), color var(--duration-fast) var(--ease-enter);
  }

  .chip:hover {
    color: var(--text);
  }

  .chip[aria-pressed='true'] {
    border-color: var(--border-strong);
    background: var(--surface-overlay);
    color: var(--text);
  }

  .chip:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-thin);
  }

  .dot {
    inline-size: var(--space-2);
    block-size: var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--text-quiet);
  }

  .error .dot {
    background: var(--status-down);
  }

  .warning .dot {
    background: var(--status-warning);
  }

  .shown {
    margin-inline-start: auto;
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    font-variant-numeric: tabular-nums;
  }
</style>
