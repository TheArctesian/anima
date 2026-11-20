<script lang="ts">
  import { accessApi } from '$lib/access/api';
  import { loadClientAt } from '$lib/access/navigation';
  import { forgetHubAddress } from '$lib/desktop/address';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { useHub } from '$lib/hub/context';
  import type { HubResult } from '$lib/hub/hub';
  import ConfirmModal from '$lib/forms/modal/ConfirmModal.svelte';
  import { signOutMobile } from '$lib/access/mobile/sign-out';

  // "Leave hub" of the phone layout (mockup 7). A phone has no link yet (mobile plan M3-1), so leaving signs out and
  // forgets the hub's address; the app starts again at the first-run choices.
  const hub = useHub();
  const api = accessApi(hub);
  let asking = $state(false);

  async function leave(): Promise<HubResult<unknown>> {
    const kept = await signOutMobile(api, hub.address);
    if (kept !== null) return { ok: false, error: { code: 'INTERNAL', message: kept } };
    forgetHubAddress(localStorage, hub.address.origin);
    loadClientAt('/welcome');
    return { ok: true, value: null };
  }
</script>

<div class="phone-leave">
  <FormButton tone="danger" onclick={() => (asking = true)}>Leave hub</FormButton>
</div>

<ConfirmModal
  open={asking}
  title="Leave this hub?"
  body="This phone signs out and forgets the hub."
  confirmLabel="Leave hub"
  onConfirm={leave}
  onCancel={() => (asking = false)}
/>
