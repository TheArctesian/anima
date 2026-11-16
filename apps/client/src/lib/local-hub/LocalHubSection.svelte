<script lang="ts">
  import { loadClientAt } from '$lib/access/navigation';
  import { saveHubKey } from '$lib/desktop/local-hub/shell';
  import { clearAccessToken } from '$lib/desktop/token';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { useHub } from '$lib/hub/context';
  import type { IconName } from '$lib/icons/icons';
  import { isMac } from '$lib/platform/mac';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import SettingsSection from '$lib/settings/SettingsSection.svelte';
  import { hubIsThisComputer } from '$lib/stay-awake/stay-awake';
  import RecoverDialog from './account/RecoverDialog.svelte';
  import type { LocalHubWatch } from './local-hub.svelte';
  import { linkText, stateText } from './state';

  interface Props {
    /** The section's anchor, heading, icon and line (SET-001). */
    section: { id: string; title: string; icon: IconName; description: string };
    hubState: LocalHubWatch;
    mac?: boolean;
  }

  const { section, hubState, mac = isMac(navigator.userAgent) }: Props = $props();
  const hub = useHub();
  let keyNote = $state<string | null>(null);
  let recovering = $state(false);
  const status = $derived(hubState.status);
  const running = $derived(status?.state === 'ready' || status?.state === 'starting');
  const stopping = $derived(status?.state === 'stopping');

  async function saveKey(): Promise<void> {
    keyNote = null;
    const saved = await saveHubKey();
    keyNote = !saved.ok ? saved.message : saved.value ? 'Saved. Keep it with your backups.' : null;
  }

  /** Every token of the operator stopped: the client signs in again when it uses this hub (FRN-007). */
  async function recovered(): Promise<void> {
    recovering = false;
    if (!hubIsThisComputer(hub.address.origin, hubState.origin)) return;
    await clearAccessToken(hub.address).catch(() => undefined);
    loadClientAt('/sign-in');
  }
</script>

<!-- Only on a computer where the hub is installed (FRN-006). -->
{#if status?.installed === true}
  <SettingsSection {...section}>
    <SettingRow label="State">
      {#snippet hint()}
        <p class="hint" aria-live="polite">{stateText(status, mac)} {linkText(status) ?? ''}</p>
      {/snippet}
      {#if running}
        <FormButton size="sm" busy={hubState.busy} onclick={() => void hubState.stop()}>Stop</FormButton>
      {:else}
        <!-- A hub that is still stopping cannot start yet: the button waits, and says so. -->
        <FormButton size="sm" busy={hubState.busy} aria-disabled={stopping} onclick={() => void hubState.start()}>{stopping ? 'Stopping…' : 'Start'}</FormButton>
      {/if}
      {#snippet message()}<p role="alert">{hubState.problem ?? ''}</p>{/snippet}
    </SettingRow>
    <SettingRow label="Hub key">
      {#snippet hint()}
        <p class="hint">The key seals the device credentials and the authenticator keys. Without it the hub cannot open its data.</p>
      {/snippet}
      <FormButton size="sm" onclick={() => void saveKey()}>Save a copy of the hub key</FormButton>
      {#snippet message()}<p role="alert">{keyNote ?? ''}</p>{/snippet}
    </SettingRow>
    <SettingRow label="Account">
      {#snippet hint()}
        <p class="hint">A new password and authenticator key, when you lost either. Every sign-in of the account ends.</p>
      {/snippet}
      <FormButton size="sm" aria-disabled={status.state !== 'ready'} onclick={() => (recovering = status.state === 'ready')}>Recover the account</FormButton>
    </SettingRow>
  </SettingsSection>
{/if}

{#if recovering}
  <RecoverDialog onrecovered={() => void recovered()} onclose={() => (recovering = false)} />
{/if}
