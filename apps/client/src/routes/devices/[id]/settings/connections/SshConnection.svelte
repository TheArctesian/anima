<script lang="ts">
  import { tick } from 'svelte';
  import type { DeviceDetail } from '$lib/devices/api';
  import type { ConnectionTestResult } from '$lib/devices/form/connection-test';
  import RemoteLoginButton from '$lib/devices/form/form/RemoteLoginButton.svelte';
  import TestConnection from '$lib/devices/form/form/TestConnection.svelte';
  import { emptyRemoteSecrets } from '$lib/devices/form/remote/values';
  import type { DeviceFormValues } from '$lib/devices/form/values';
  import SaveMark from '$lib/forms/feedback/SaveMark.svelte';
  import Segmented from '$lib/forms/segmented/Segmented.svelte';
  import type { ConnectionTestInput, SshAuth } from '$lib/hub/generated/graphql';
  import type { HubResult } from '$lib/hub/hub';
  import Icon from '$lib/icons/Icon.svelte';
  import type { SettingsCalls } from '../calls';
  import type { SettingsDraft } from '../draft.svelte';
  import ConnectionRow from './ConnectionRow.svelte';
  import { isSetUp, summaryParts } from './connections';
  import FieldErrors from './FieldErrors.svelte';
  import SecretRow, { type TypedSecret } from './SecretRow.svelte';
  import TextCell from './TextCell.svelte';

  interface Props {
    draft: SettingsDraft;
    device: DeviceDetail;
    /** The device as the hub last stored it, for the connection test. */
    saved: DeviceFormValues;
    calls: SettingsCalls;
    onTest: (input: ConnectionTestInput) => Promise<HubResult<ConnectionTestResult>>;
    open: boolean;
  }

  let { draft, device, saved, calls, onTest, open = $bindable() }: Props = $props();
  /** "Set up" was pressed: the row shows its switch, off, while the operator fills it in. */
  let settingUp = $state(false);
  let root = $state<HTMLElement>();

  const AUTH: readonly { value: SshAuth; label: string }[] = [
    { value: 'PASSWORD', label: 'Password' },
    { value: 'PRIVATE_KEY', label: 'Private key' },
  ];

  async function setUp(): Promise<void> {
    settingUp = true;
    open = true;
    await tick();
    root?.querySelector<HTMLElement>('#device-ssh-username')?.focus();
  }

  function storeSecret(secret: TypedSecret): Promise<string | null> {
    return draft.saveSecret('ssh.credential', { credential: secret, remoteSecrets: emptyRemoteSecrets() });
  }
</script>

{#snippet summary()}{isSetUp(device, 'ssh') || settingUp ? summaryParts(draft.values, 'ssh').join(' · ') : 'Not set up'}{/snippet}

<div bind:this={root}>
  <ConnectionRow
    protocol="ssh"
    name="SSH"
    icon="carbon:terminal"
    {summary}
    bind:enabled={draft.values.ssh.enabled}
    setUp={settingUp || isSetUp(device, 'ssh')}
    bind:open
    mark={draft.marks['ssh.enabled']}
    onToggle={() => void draft.commit('ssh.enabled')}
    onSetUp={() => void setUp()}
  >
    <div class="fields">
      <TextCell {draft} key="ssh.port" label="Port" port />
      <TextCell {draft} key="ssh.username" label="User name" />
      <div class="wide">
        <span class="label">Sign in with<SaveMark mark={draft.marks['ssh.auth']} /></span>
        <Segmented label="Sign in with" options={AUTH} bind:value={draft.values.ssh.auth} onchange={() => void draft.commit('ssh.auth')} />
      </div>
      <SecretRow
        label={saved.ssh.auth === 'PASSWORD' ? 'Password' : 'Private key'}
        id={saved.ssh.auth === 'PASSWORD' ? 'device-credential-password' : 'device-credential-privateKey-key'}
        kind={saved.ssh.auth === 'PASSWORD' ? 'password' : 'key'}
        stored={device.ssh.credentialStored}
        unavailable={saved.ssh.enabled ? null : 'Turn SSH on to store one.'}
        mark={draft.marks['ssh.credential']}
        onSave={storeSecret}
        onRemove={calls.onClearCredential}
      />
      <div class="wide">
        <span class="label">Host key</span>
        <div class="key">
          {#if device.ssh.hostKey}
            <span class="trusted"><Icon name="carbon:checkmark" size="sm" />Trusted</span>
            <span class="fingerprint">{device.ssh.hostKey.algorithm} · {device.ssh.hostKey.fingerprintSha256}</span>
          {:else}
            <span class="quiet">None accepted yet. Test the connection to see it.</span>
          {/if}
        </div>
      </div>
      <FieldErrors errors={draft.errorsIn('ssh').filter((error) => !error.field.startsWith('credential.'))} />
    </div>
    {#if device.os === 'MACOS'}
      <div class="foot"><RemoteLoginButton /></div>
    {/if}
    {#if draft.values.ssh.enabled}
      <div class="foot">
        <TestConnection initial={saved} values={draft.values} deviceId={device.id} canTest={() => draft.checkForTest('ssh')} {onTest} onAcceptHostKey={calls.onAcceptHostKey} />
      </div>
    {/if}
  </ConnectionRow>
</div>

<style>
  .fields {
    display: grid;
    grid-template-columns: calc(var(--space-16) * 1.75) minmax(0, 1fr);
    gap: var(--space-4) var(--space-4);
  }

  .wide {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-1);
    grid-column: 1 / -1;
    min-inline-size: 0;
  }

  .label {
    display: flex;
    align-items: baseline;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .key {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    inline-size: 100%;
    min-block-size: var(--control-md);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }

  .trusted {
    display: inline-flex;
    flex: none;
    align-items: center;
    gap: var(--space-1);
    color: var(--status-up-text);
  }

  .fingerprint {
    min-inline-size: 0;
    overflow: hidden;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .quiet {
    color: var(--text-tertiary);
  }

  .foot {
    margin-block-start: var(--space-4);
  }
</style>
