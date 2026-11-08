<script lang="ts">
  import type { DeviceCheck } from '$lib/checks/api';
  import CheckForm from '$lib/checks/form/CheckForm.svelte';
  import { siblingOf } from '$lib/checks/form/model/group';
  import type { CheckFormValues } from '$lib/checks/form/model/values';
  import Modal from '$lib/forms/modal/Modal.svelte';
  import type { HubResult } from '$lib/hub/hub';

  interface Props {
    open: boolean;
    /** Null to add a check. */
    check: DeviceCheck | null;
    /** The device's checks; a group picks its children among them. */
    checks: readonly DeviceCheck[];
    /** The route builds the input and calls the hub; the form calls `onClose` once the check and its channels are saved. */
    onSave: (values: CheckFormValues) => Promise<HubResult<unknown>>;
    onClose: () => void;
  }

  const { open, check, checks, onSave, onClose }: Props = $props();
  const siblings = $derived(checks.map(siblingOf));
</script>

<Modal {open} title={check === null ? 'Add check' : `Edit ${check.name}`} {onClose} size="form">
  <CheckForm {check} {siblings} {onSave} onDone={onClose} onCancel={onClose} />
</Modal>
