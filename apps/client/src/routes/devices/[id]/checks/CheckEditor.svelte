<script lang="ts">
  import type { DeviceCheck } from '$lib/checks/api';
  import CheckForm from '$lib/forms/check/CheckForm.svelte';
  import { siblingOf } from '$lib/forms/check/model/group';
  import type { CheckFormValues } from '$lib/forms/check/model/values';
  import Modal from '$lib/forms/modal/Modal.svelte';
  import type { HubResult } from '$lib/hub/hub';

  interface Props {
    open: boolean;
    /** Null to add a check. */
    check: DeviceCheck | null;
    /** The device's checks; a group picks its children among them. */
    checks: readonly DeviceCheck[];
    /** The route builds the input and calls the hub; on success it closes the editor. */
    onSave: (values: CheckFormValues) => Promise<HubResult<unknown>>;
    onClose: () => void;
  }

  const { open, check, checks, onSave, onClose }: Props = $props();
  const siblings = $derived(checks.map(siblingOf));
</script>

<Modal {open} title={check === null ? 'Add check' : `Edit ${check.name}`} {onClose}>
  <CheckForm {check} {siblings} {onSave} onCancel={onClose} />
</Modal>
