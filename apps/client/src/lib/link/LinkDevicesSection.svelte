<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import ConfirmModal from '$lib/forms/modal/ConfirmModal.svelte';
  import Modal from '$lib/forms/modal/Modal.svelte';
  import { useHub } from '$lib/hub/context';
  import type { HubError, HubResult } from '$lib/hub/hub';
  import type { IconName } from '$lib/icons/icons';
  import { createClock } from '$lib/nav/clock.svelte';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import SettingsSection from '$lib/settings/SettingsSection.svelte';
  import { linkApi, type LinkApi, type LinkDevice } from './api';
  import DeviceRow from './DeviceRow.svelte';
  import { devicesShown } from './devices';
  import InviteMaker from './InviteMaker.svelte';

  interface Props {
    section: { id: string; title: string; icon: IconName; description: string };
    /** The clock of the invite countdown; tests pass their own (CQ-021). */
    clock?: { readonly current: Date };
    api?: LinkApi;
  }

  const hub = useHub();
  const { section, clock = createClock(1000), api = linkApi(hub) }: Props = $props();
  let devices = $state<readonly LinkDevice[]>([]);
  let answer = $state<{ ok: true } | { ok: false; error: HubError } | null>(null);
  let inviting = $state(false);
  let revoking = $state<LinkDevice | null>(null);
  const shown = $derived(devicesShown(answer));

  async function load(): Promise<void> {
    const result = await api.devices();
    if (result.ok) devices = result.value;
    answer = result.ok ? { ok: true } : { ok: false, error: result.error };
  }

  $effect(() => {
    void load();
  });

  async function revoke(): Promise<HubResult<unknown>> {
    if (revoking === null) return { ok: true, value: undefined };
    const result = await api.revoke(revoking.key);
    if (result.ok) {
      revoking = null;
      await load();
    }
    return result;
  }
</script>

<!-- Hidden on a hub with no link (the VPS mode answers UNAVAILABLE) (FRN-009). -->
{#if shown === 'listed' || shown === 'failed'}
  <SettingsSection {...section}>
    <SettingRow label="New device">
      {#snippet hint()}<p class="hint">Works once. Expires after 15 minutes.</p>{/snippet}
      <FormButton tabindex={0} onclick={() => (inviting = true)}>Invite device</FormButton>
    </SettingRow>
    <div class="list">
      {#if shown === 'failed'}
        <p role="alert">Can't load the link's devices.</p>
        <FormButton size="sm" onclick={() => void load()}>Retry</FormButton>
      {:else if devices.length === 0}
        <p class="quiet">No devices yet.</p>
      {:else}
        <ul>
          {#each devices as device (device.key)}
            <DeviceRow {device} onrevoke={() => (revoking = device)} />
          {/each}
        </ul>
      {/if}
    </div>
  </SettingsSection>
{/if}

{#if inviting}
  <Modal open title="Invite device" preset="md" closeOnScrim={false} dismissible onClose={() => { inviting = false; void load(); }}>
    <InviteMaker {api} now={clock.current} />
  </Modal>
{/if}

{#key revoking?.key}
  <ConfirmModal
    open={revoking !== null}
    title="Revoke {revoking?.name ?? ''}?"
    body="Its link and open sessions end now. Rejoining needs a new invite."
    confirmLabel="Revoke {revoking?.name ?? ''}"
    onConfirm={revoke}
    onCancel={() => (revoking = null)}
  />
{/key}

<style>
  .list {
    display: grid;
    gap: var(--space-2);
    padding: var(--space-3) var(--space-1);
    border-block-start: var(--border-thin) solid var(--border-subtle);
  }

  ul {
    display: grid;
    gap: var(--space-2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  p {
    margin: 0;
    font-size: var(--text-sm);
  }

  .quiet {
    color: var(--text-secondary);
  }
</style>
