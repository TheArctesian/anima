<script lang="ts">
  import type { DeviceDetail } from '$lib/devices/api';
  import type { ConnectionTestResult } from '$lib/devices/form/connection-test';
  import DeviceForm from '$lib/devices/form/form/DeviceForm.svelte';
  import type { DeviceFields } from '$lib/devices/form/values';
  import FormButton from '$lib/forms/FormButton.svelte';
  import WebInterfaces from '$lib/devices/form/web/WebInterfaces.svelte';
  import ConfirmModal from '$lib/forms/modal/ConfirmModal.svelte';
  import type { ConnectionTestInput } from '$lib/hub/generated/graphql';
  import type { HubResult } from '$lib/hub/hub';
  import AgentPanel from '$lib/metrics/agent/AgentPanel.svelte';
  import type { AgentCalls } from '$lib/metrics/agent/install.svelte';
  import type { ThisComputerShell } from '$lib/desktop/agent/shell';
  import type { SettingsSection } from '../tabs/tabs';
  import type { SettingsCalls } from './calls';

  interface Props {
    device: DeviceDetail;
    /** Counts the saves; the form starts again from the saved device after each, so it never compares with stale values. */
    revision: number;
    section: SettingsSection | null;
    /** The hub calls of the form and the web interfaces (WEB-001, WEB-002, DEV-011, SEC-007). */
    calls: SettingsCalls;
    /** The Install agent panel's hub calls and the hub origin the agent must reach (MET-019). */
    agent: { calls: AgentCalls; origin: string; reveal: boolean; shell: ThisComputerShell | null };
    now: () => Date;
    onTest: (input: ConnectionTestInput) => Promise<HubResult<ConnectionTestResult>>;
    /** Discards the edits: the form starts again from the saved device. */
    onCancel: () => void;
    /** DEV-014: runs only after the confirmation; on success the route goes to the device list. */
    onDelete: () => Promise<HubResult<unknown>>;
  }

  const { device, revision, section, calls, agent, now, onTest, onCancel, onDelete }: Props = $props();
  /** The device as the fields the form edits (D-3). */
  const fields: DeviceFields = $derived(device);
  let confirming = $state(false);
</script>

<div class="settings">
  {#key revision}
    <DeviceForm
      device={fields}
      deviceId={device.id}
      credentialStored={device.ssh.credentialStored}
      rdpCredentialStored={fields.rdp?.credentialStored ?? false}
      vncCredentialStored={fields.vnc?.credentialStored ?? false}
      section={section ?? undefined}
      onSave={(values) => calls.onSave(values)}
      {onTest}
      onAcceptHostKey={calls.onAcceptHostKey}
      onAcceptServerCertificate={calls.onAcceptServerCertificate}
      onClearCredential={calls.onClearCredential}
      onClearRdpCredential={calls.onClearRdpCredential}
      onClearVncCredential={calls.onClearVncCredential}
      {onCancel}
    />
  {/key}
  <p class="note">
    New devices get one ping check with the defaults of Section 6.4: every 60 seconds, 3 retries 20 seconds apart, and Degraded
    above 1000 ms. Change the checks in the Checks tab.
  </p>
  <WebInterfaces interfaces={device.webInterfaces} suggestions={device.webInterfaceSuggestions} calls={calls.webInterfaces} />
  <AgentPanel deviceId={device.id} deviceName={device.name} host={device.host} source={device.metricsSource} calls={agent.calls} origin={agent.origin} reveal={agent.reveal} shell={agent.shell} {now} />
  <section class="danger" aria-labelledby="device-delete-heading">
    <h3 id="device-delete-heading">Delete device</h3>
    <p>Its credentials, checks and history are deleted too.</p>
    <FormButton tone="danger" onclick={() => { confirming = true; }}>Delete device</FormButton>
  </section>
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
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
    max-inline-size: calc(var(--space-16) * 10);
  }

  .note {
    margin: 0;
    color: var(--text-muted);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  .danger {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
    padding-block-start: var(--space-4);
    border-block-start: var(--border-thin) solid var(--border);
  }

  h3 {
    margin: 0;
    font-size: var(--text-md);
  }

  .danger p {
    margin: 0;
    color: var(--text-muted);
  }
</style>
