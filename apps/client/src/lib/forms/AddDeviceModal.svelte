<script lang="ts">
  import type { HubResult } from '$lib/hub/error';
  import type { AddedDevice } from './add-device';
  import DeviceForm from './device/form/DeviceForm.svelte';
  import type { ConnectionTestInput, ConnectionTestResult, SiteOption } from './device/hub-types';
  import type { DeviceFormValues } from './device/values';
  import Modal from './modal/Modal.svelte';

  interface Props {
    open: boolean;
    sites: readonly SiteOption[];
    /** The route runs `addDevice` (D-6). */
    onCreate: (values: DeviceFormValues, siteId: string) => Promise<HubResult<AddedDevice>>;
    onTest: (input: ConnectionTestInput) => Promise<HubResult<ConnectionTestResult>>;
    onClose: () => void;
    /** Called once the device exists; `credentialError` is set when its credential was not stored. */
    onCreated: (added: AddedDevice) => void;
  }

  const { open, sites, onCreate, onTest, onClose, onCreated }: Props = $props();

  async function create(values: DeviceFormValues, siteId: string | null): Promise<HubResult<AddedDevice>> {
    const result = await onCreate(values, siteId ?? '');
    if (result.ok) onCreated(result.value);
    return result;
  }
</script>

<Modal {open} title="Add device" {onClose}>
  <DeviceForm
    device={null}
    {sites}
    onSave={create}
    {onTest}
    onAcceptHostKey={() => Promise.resolve({ ok: true, value: undefined })}
    onClearCredential={() => Promise.resolve({ ok: true, value: undefined })}
    onCancel={onClose}
  />
</Modal>
