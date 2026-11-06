<script lang="ts">
  import type { DeviceDetail } from '$lib/devices/api';
  import type { ConnectionTestResult } from '$lib/forms/device/connection-test';
  import DeviceForm from '$lib/forms/device/form/DeviceForm.svelte';
  import type { DeviceFormValues } from '$lib/forms/device/values';
  import FormButton from '$lib/forms/FormButton.svelte';
  import WebInterfaces, { type WebInterfaceCalls } from '$lib/forms/device/web/WebInterfaces.svelte';
  import type { WebInterface, WebInterfaceSuggestion } from '$lib/forms/hub-pending';
  import ConfirmModal from '$lib/forms/modal/ConfirmModal.svelte';
  import type { ConnectionTestInput } from '$lib/hub/generated/graphql';
  import type { HubResult } from '$lib/hub/hub';
  import type { SettingsSection } from '../tabs/tabs';

  interface Props {
    device: DeviceDetail;
    /** Counts the saves; the form starts again from the saved device after each, so it never compares with stale values. */
    revision: number;
    section: SettingsSection | null;
    onSave: (values: DeviceFormValues) => Promise<HubResult<unknown>>;
    onTest: (input: ConnectionTestInput) => Promise<HubResult<ConnectionTestResult>>;
    onAcceptHostKey: (fingerprint: string) => Promise<HubResult<unknown>>;
    onClearCredential: () => Promise<HubResult<unknown>>;
    /** Discards the edits: the form starts again from the saved device. */
    onCancel: () => void;
    /** DEV-014: runs only after the confirmation; on success the route goes to the device list. */
    onDelete: () => Promise<HubResult<unknown>>;
    /** WEB-001, WEB-002, D-4: the web interfaces of the device and the calls on them; without it the section is not shown. */
    webInterfaces?: { interfaces: readonly WebInterface[]; suggestions: readonly WebInterfaceSuggestion[]; calls: WebInterfaceCalls };
  }

  const { device, revision, section, onSave, onTest, onAcceptHostKey, onClearCredential, onCancel, onDelete, webInterfaces }: Props = $props();
  let confirming = $state(false);
</script>

<div class="settings">
  {#key revision}
    <DeviceForm
      {device}
      deviceId={device.id}
      credentialStored={device.ssh.credentialStored}
      section={section ?? undefined}
      onSave={(values) => onSave(values)}
      {onTest}
      {onAcceptHostKey}
      {onClearCredential}
      {onCancel}
    />
  {/key}
  <p class="note">
    New devices get one ping check with the defaults of Section 6.4: every 60 seconds, 3 retries 20 seconds apart, and Degraded
    above 1000 ms. Change the checks in the Checks tab.
  </p>
  {#if webInterfaces !== undefined}
    <WebInterfaces {...webInterfaces} />
  {/if}
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
