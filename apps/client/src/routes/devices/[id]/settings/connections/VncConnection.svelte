<script lang="ts">
  import { tick } from 'svelte';
  import type { DeviceDetail } from '$lib/devices/api';
  import RemoteTest from '$lib/devices/form/remote/test/RemoteTest.svelte';
  import type { RemoteTestInput, RemoteTestResult } from '$lib/devices/form/remote/test/remote-test';
  import { vncWarning } from '$lib/devices/form/remote/warnings';
  import type { DeviceFormValues } from '$lib/devices/form/values';
  import type { HubResult } from '$lib/hub/hub';
  import Icon from '$lib/icons/Icon.svelte';
  import type { SettingsCalls } from '../calls';
  import type { SettingsDraft } from '../draft.svelte';
  import ConnectionRow from './ConnectionRow.svelte';
  import { isSetUp, summaryParts } from './connections';
  import FieldErrors from './FieldErrors.svelte';
  import SecretRow from './SecretRow.svelte';
  import TextCell from './TextCell.svelte';

  interface Props {
    draft: SettingsDraft;
    device: DeviceDetail;
    saved: DeviceFormValues;
    calls: SettingsCalls;
    onTest: (input: RemoteTestInput) => Promise<HubResult<RemoteTestResult>>;
    open: boolean;
  }

  let { draft, device, saved, calls, onTest, open = $bindable() }: Props = $props();
  let settingUp = $state(false);
  let root = $state<HTMLElement>();
  /** RA-020: the hub's warning on the saved VNC settings, as one sentence. */
  const warning = $derived(vncWarning(device.accessWarnings));

  async function setUp(): Promise<void> {
    settingUp = true;
    open = true;
    await tick();
    root?.querySelector<HTMLElement>('#device-vnc-port')?.focus();
  }
</script>

{#snippet summary()}
  {#if isSetUp(device, 'vnc') || settingUp}
    {summaryParts(draft.values, 'vnc').join(' · ')}{#if warning !== null}<span class="warned">&nbsp;·&nbsp;not encrypted</span>{/if}
  {:else}
    Not set up
  {/if}
{/snippet}

<div bind:this={root}>
  <ConnectionRow
    protocol="vnc"
    name="VNC"
    icon="carbon:screen-map"
    {summary}
    bind:enabled={draft.values.vnc.enabled}
    setUp={settingUp || isSetUp(device, 'vnc')}
    bind:open
    mark={draft.marks['vnc.enabled']}
    onToggle={() => void draft.commit('vnc.enabled')}
    onSetUp={() => void setUp()}
  >
    <div class="fields">
      <TextCell {draft} key="vnc.port" label="Port" port />
      <TextCell {draft} key="vnc.username" label="User name" about="Optional. macOS Screen Sharing needs one." />
      <SecretRow
        label="Password"
        id="device-vnc-password"
        kind="password"
        stored={device.vnc.credentialStored}
        unavailable={saved.vnc.enabled ? null : 'Turn VNC on to store one.'}
        mark={draft.marks['vnc.password']}
        onSave={(secret) => draft.saveSecret('vnc.password', { credential: saved.credential, remoteSecrets: { rdpPassword: '', vncPassword: secret.password } })}
        onRemove={calls.onClearVncCredential}
      />
      <FieldErrors errors={draft.errorsIn('vnc')} />
    </div>
    {#if warning !== null}
      <p class="warning" role="note"><Icon name="carbon:warning-alt" size="sm" />{warning}</p>
    {/if}
    {#if draft.values.vnc.enabled}
      <div class="foot">
        <RemoteTest protocol="VNC" initial={saved} values={draft.values} deviceId={device.id} canTest={() => draft.checkForTest('vnc')} {onTest} />
      </div>
    {/if}
  </ConnectionRow>
</div>

<style>
  .fields {
    display: grid;
    grid-template-columns: calc(var(--space-16) * 1.75) minmax(0, 1fr);
    gap: var(--space-4);
  }

  .warned {
    color: var(--status-warning-text);
  }

  .warning {
    display: flex;
    align-items: flex-start;
    gap: var(--space-2);
    margin: var(--space-4) 0 0;
    color: var(--status-warning-text);
    font-size: var(--text-sm);
  }

  .foot {
    margin-block-start: var(--space-4);
  }
</style>
