<script lang="ts">
  import { onMount } from 'svelte';
  import { forgetListener } from '$lib/access/listener/check';
  import { loadClientAt } from '$lib/access/navigation';
  import { forgetHubAddress } from '$lib/desktop/address';
  import { leaveLink, readLink, type DeviceLinkStatus } from '$lib/desktop/link/shell';
  import { clearAccessToken } from '$lib/desktop/token';
  import FormButton from '$lib/forms/FormButton.svelte';
  import ConfirmModal from '$lib/forms/modal/ConfirmModal.svelte';
  import type { HubAddress, HubResult } from '$lib/hub/hub';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import { deviceLinkAddress, deviceLinkText } from './devices';

  // The link of a joined device to its hub (LNK-006, FRN-005): shown only when this device has joined one.
  let status = $state<DeviceLinkStatus | null>(null);
  let leaving = $state(false);
  /** The link's address once this device has left: a retry only forgets what was kept for it. */
  let left = $state<{ address: HubAddress | null } | null>(null);
  const NOT_CLEARED = 'This device left the hub, but the key store kept the access token for the link address. Retry.';

  async function read(): Promise<void> {
    const result = await readLink();
    status = result.ok ? result.value : null;
  }

  onMount(() => {
    void read();
  });

  /**
   * Leaves the hub, then forgets everything this device kept for the link's address: the listener answer, the access
   * token and the saved address (M-C). The app starts again at the first-run choices.
   */
  async function leave(): Promise<HubResult<unknown>> {
    if (left === null) {
      const address = status?.port === undefined ? null : deviceLinkAddress(status.port);
      const result = await leaveLink();
      if (!result.ok) return { ok: false, error: { code: 'INTERNAL', message: result.message } };
      left = { address };
    }
    const { address } = left;
    if (address !== null) {
      forgetListener(address.origin);
      try {
        await clearAccessToken(address);
      } catch {
        // The device has left; the token must still go before the app starts again, so the person retries.
        return { ok: false, error: { code: 'INTERNAL', message: NOT_CLEARED } };
      }
      forgetHubAddress(localStorage, address.origin);
    }
    loadClientAt('/welcome');
    return { ok: true, value: null };
  }
</script>

{#if status?.joined === true}
  {@const text = deviceLinkText(status)}
  <SettingRow label="Link to the hub">
    {#snippet hint()}<p class="hint" aria-live="polite">{text}</p>{/snippet}
    <FormButton size="sm" tone="danger" onclick={() => (leaving = true)}>Leave this hub</FormButton>
  </SettingRow>
{/if}

<ConfirmModal
  open={leaving}
  title="Leave this hub?"
  body="This device stops its link and removes its device key. To come back it needs a new invite."
  confirmLabel={left === null ? 'Leave this hub' : 'Retry'}
  onConfirm={leave}
  onCancel={() => (leaving = false)}
/>
