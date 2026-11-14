<script lang="ts">
  import { tick } from 'svelte';
  import type { DeviceDetail } from '$lib/devices/api';
  import type { ConnectionTestResult } from '$lib/devices/form/connection-test';
  import { formValues } from '$lib/devices/form/values';
  import WebInterfaces from '$lib/devices/form/web/WebInterfaces.svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import ConfirmModal from '$lib/forms/modal/ConfirmModal.svelte';
  import type { ConnectionTestInput } from '$lib/hub/generated/graphql';
  import type { HubResult } from '$lib/hub/hub';
  import Icon from '$lib/icons/Icon.svelte';
  import type { AgentCalls } from '$lib/metrics/agent/install.svelte';
  import type { ThisComputerShell } from '$lib/desktop/agent/shell';
  import PageIndex from '$lib/page/contents/PageIndex.svelte';
  import type { SettingsSection } from '../tabs/tabs';
  import type { SettingsCalls } from './calls';
  import ConnectionsSection from './connections/ConnectionsSection.svelte';
  import DeviceSection from './device/DeviceSection.svelte';
  import MonitoringSection from './device/MonitoringSection.svelte';
  import SessionsSection from './device/SessionsSection.svelte';
  import { SettingsDraft, type Later } from './draft.svelte';
  import { INDEX, SECTIONS } from './sections';

  interface Props {
    device: DeviceDetail;
    section: SettingsSection | null;
    /** The hub calls of the settings and the web interfaces (WEB-001, WEB-002, DEV-011, SEC-007). */
    calls: SettingsCalls;
    /** The Install agent panel's hub calls and the hub origin the agent must reach (MET-019). */
    agent: { calls: AgentCalls; origin: string; reveal: boolean; shell: ThisComputerShell | null };
    now: () => Date;
    onTest: (input: ConnectionTestInput) => Promise<HubResult<ConnectionTestResult>>;
    /** Opens the Checks tab. */
    onOpenChecks: () => void;
    /** DEV-014: runs only after the confirmation; on success the route goes to the device list. */
    onDelete: () => Promise<HubResult<unknown>>;
    /** How "Saved" waits before it fades; tests pass their own clock. */
    later?: Later;
  }

  const { device, section, calls, agent, now, onTest, onOpenChecks, onDelete, later }: Props = $props();
  /** The device as the hub last stored it, in the shape the settings edit (D-3). */
  const saved = $derived(formValues(device));
  // The draft lives as long as the tab; it reads the saved device afresh for each change.
  // svelte-ignore state_referenced_locally
  const draft = new SettingsDraft(() => saved, (values) => calls.onSave(values), later);
  let confirming = $state(false);
  let root = $state<HTMLElement>();

  // DEV-022: the General place is the Device section; its first field takes focus.
  $effect(() => {
    if (section !== 'general') return;
    void tick().then(() => {
      const name = root?.querySelector<HTMLElement>('#device-name') ?? null;
      name?.focus();
      if (typeof name?.scrollIntoView === 'function') name.scrollIntoView({ block: 'nearest' });
    });
  });
</script>

<!-- The device's settings in one column of groups beside an index (UR-7, D17, DEV-028). Every change saves as it is
     made; there is no Save or Cancel. -->
<div class="settings" bind:this={root}>
  <div class="column">
    <p class="how">
      <span class="quiet"><Icon name="carbon:cloud-upload" size="sm" /></span>Changes save as you make them. Text saves when you leave the field or press Return.
    </p>
    <div class="groups">
    <DeviceSection {draft} />
    <MonitoringSection {draft} {device} {agent} {now} {onOpenChecks} />
    <ConnectionsSection {draft} {device} {saved} {calls} {section} {onTest} />
    <SessionsSection {draft} deviceId={device.id} />
    <div class="web" id={SECTIONS.web.id} data-section={SECTIONS.web.id}>
      <WebInterfaces interfaces={device.webInterfaces} suggestions={device.webInterfaceSuggestions} calls={calls.webInterfaces} />
    </div>
    <section class="danger" aria-labelledby="device-delete-heading">
      <h3 id="device-delete-heading">Delete device</h3>
      <p>Its credentials, checks and history are deleted too.</p>
      <FormButton tone="danger" onclick={() => { confirming = true; }}>Delete device</FormButton>
    </section>
    </div>
  </div>
  <PageIndex entries={INDEX} label="On this page" heading="On this page" narrow="hidden" />
</div>

<ConfirmModal
  open={confirming}
  title="Delete {device.name}?"
  body="Anima deletes {device.name} with its credentials, checks, heartbeats and metrics. This cannot be undone."
  confirmLabel="Delete {device.name}"
  onConfirm={onDelete}
  onCancel={() => {
    confirming = false;
  }}
/>

<style>
  .settings {
    display: grid;
    grid-template-columns: minmax(0, calc(var(--content-form) + var(--space-8))) calc(var(--space-16) * 3);
    gap: var(--space-12);
    align-items: start;
  }

  .column {
    display: flex;
    flex-direction: column;
    gap: var(--space-5);
    min-inline-size: 0;
  }

  .groups {
    display: flex;
    flex-direction: column;
    gap: var(--space-10);
  }

  .how {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    color: var(--text-tertiary);
    font-size: var(--text-sm);
  }

  .quiet {
    display: inline-flex;
    color: var(--text-quiet);
  }

  .web {
    scroll-margin-block-start: var(--space-6);
  }

  .danger {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
    padding-block-start: var(--space-4);
    border-block-start: var(--border-thin) solid var(--border-strong);
  }

  h3 {
    margin: 0;
    font-size: var(--text-md);
  }

  .danger p {
    margin: 0;
    color: var(--text-secondary);
  }

  @media (max-width: 1100.98px) {
    .settings {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
