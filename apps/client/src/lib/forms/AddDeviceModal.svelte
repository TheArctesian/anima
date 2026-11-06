<script lang="ts">
  import type { ThisMachine } from '$lib/desktop/thisMachine';
  import type { HubResult } from '$lib/hub/error';
  import type { ConnectionTestInput } from '$lib/hub/generated/graphql';
  import type { AddedDevice } from './add-device';
  import type { SitePlacement } from './device/site';
  import DeviceForm from './device/form/DeviceForm.svelte';
  import type { ConnectionTestResult } from './device/connection-test';
  import { thisMachineValues, type DeviceFormValues, type SiteOption } from './device/values';
  import FormButton from './FormButton.svelte';
  import MachineChoice from './MachineChoice.svelte';
  import Modal from './modal/Modal.svelte';
  import { dontAskLabel, promptTitle, type MachineChoice as Choice } from './this-machine';

  interface Props {
    open: boolean;
    sites: readonly SiteOption[];
    /**
     * The computer the app runs on, when it is known and not yet a device (R-060): the modal offers it first,
     * pre-filled. Null for the empty form alone.
     */
    machine?: ThisMachine | null;
    /** The app opened the modal itself to ask about `machine`; `onDontAsk` turns that off for good. */
    asking?: boolean;
    onDontAsk?: () => void;
    /** The route runs `addDevice` (D-6). */
    onCreate: (values: DeviceFormValues, site: SitePlacement) => Promise<HubResult<AddedDevice>>;
    onTest: (input: ConnectionTestInput) => Promise<HubResult<ConnectionTestResult>>;
    onClose: () => void;
    /** Called once the device exists; `credentialError` is set when its credential was not stored. */
    onCreated: (added: AddedDevice) => void;
  }

  const {
    open,
    sites,
    machine = null,
    asking = false,
    onDontAsk = () => undefined,
    onCreate,
    onTest,
    onClose,
    onCreated,
  }: Props = $props();
  let chosen = $state<Choice>('this');

  // Each opening offers this computer first.
  $effect.pre(() => {
    if (open) chosen = 'this';
  });

  const prefilled = $derived(machine !== null && chosen === 'this' ? machine : null);
  const title = $derived(asking && machine !== null ? promptTitle(machine.os) : 'Add device');

  async function create(values: DeviceFormValues, site: SitePlacement | null): Promise<HubResult<AddedDevice>> {
    // The form is in create mode here, so it always names a site.
    const result = await onCreate(values, site ?? { kind: 'existing', id: '' });
    if (result.ok) onCreated(result.value);
    return result;
  }
</script>

<Modal {open} {title} {onClose} size="form">
  {#if machine !== null}
    <MachineChoice
      os={machine.os}
      {chosen}
      onChoose={(choice) => {
        chosen = choice;
      }}
    />
  {/if}
  {#key prefilled}
    <DeviceForm
      device={null}
      initial={prefilled === null ? undefined : thisMachineValues(prefilled)}
      remoteLoginOff={prefilled !== null && !prefilled.sshListening}
      {sites}
      onSave={create}
      {onTest}
      onAcceptHostKey={() => Promise.resolve({ ok: true, value: undefined })}
      onClearCredential={() => Promise.resolve({ ok: true, value: undefined })}
      onCancel={onClose}
    />
  {/key}
  {#if asking && machine !== null}
    <div class="ask">
      <FormButton onclick={onDontAsk}>{dontAskLabel(machine.os)}</FormButton>
    </div>
  {/if}
</Modal>

<style>
  .ask {
    display: flex;
    justify-content: flex-start;
  }
</style>
