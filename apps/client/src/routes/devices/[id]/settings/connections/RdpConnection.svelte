<script lang="ts">
  import { tick } from 'svelte';
  import type { DeviceDetail } from '$lib/devices/api';
  import { inputId } from '$lib/devices/form/form/errors';
  import { RDP_FLAGS, RDP_RESIZE, RDP_SECURITY } from '$lib/devices/form/remote/choices';
  import RemoteTest from '$lib/devices/form/remote/test/RemoteTest.svelte';
  import type { RemoteTestInput, RemoteTestResult } from '$lib/devices/form/remote/test/remote-test';
  import { rdpWarnings, validUntil } from '$lib/devices/form/remote/warnings';
  import type { DeviceFormValues } from '$lib/devices/form/values';
  import SaveMark from '$lib/forms/feedback/SaveMark.svelte';
  import Select from '$lib/forms/input/Select.svelte';
  import Switch from '$lib/forms/switch/Switch.svelte';
  import type { HubResult } from '$lib/hub/hub';
  import type { SettingsCalls } from '../calls';
  import type { SettingsDraft } from '../draft.svelte';
  import type { SettingKey } from '../groups';
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
  const warnings = $derived(rdpWarnings(draft.values.rdp));
  const certificate = $derived(device.rdp.serverCertificate);

  async function setUp(): Promise<void> {
    settingUp = true;
    open = true;
    await tick();
    root?.querySelector<HTMLElement>('#device-rdp-username')?.focus();
  }

  /** A select saves when it is left or Return is pressed, as text does (D17). */
  function onSelectKey(event: KeyboardEvent, key: SettingKey): void {
    if (event.key === 'Enter') void draft.commit(key);
  }
</script>

{#snippet summary()}{isSetUp(device, 'rdp') || settingUp ? summaryParts(draft.values, 'rdp').join(' · ') : 'Not set up'}{/snippet}

<div bind:this={root}>
  <ConnectionRow
    protocol="rdp"
    name="Remote Desktop (RDP)"
    icon="carbon:remote-connection"
    {summary}
    bind:enabled={draft.values.rdp.enabled}
    setUp={settingUp || isSetUp(device, 'rdp')}
    bind:open
    mark={draft.marks['rdp.enabled']}
    onToggle={() => void draft.commit('rdp.enabled')}
    onSetUp={() => void setUp()}
  >
    <div class="fields">
      <TextCell {draft} key="rdp.port" label="Port" port />
      <TextCell {draft} key="rdp.username" label="User name" />
      <TextCell {draft} key="rdp.domain" label="Domain" placeholder="Optional" />
      <SecretRow
        label="Password"
        id="device-rdp-password"
        kind="password"
        stored={device.rdp.credentialStored}
        unavailable={saved.rdp.enabled ? null : 'Turn RDP on to store one.'}
        mark={draft.marks['rdp.password']}
        onSave={(secret) => draft.saveSecret('rdp.password', { credential: saved.credential, remoteSecrets: { rdpPassword: secret.password, vncPassword: '' } })}
        onRemove={calls.onClearRdpCredential}
      />
      <div class="cell">
        <span class="label"><label for={inputId('rdp.resizeMethod')}>When the window is resized</label><SaveMark mark={draft.marks['rdp.resizeMethod']} /></span>
        <Select id={inputId('rdp.resizeMethod')} bind:value={draft.values.rdp.resizeMethod} onblur={() => void draft.commit('rdp.resizeMethod')} onkeydown={(event) => { onSelectKey(event, 'rdp.resizeMethod'); }}>
          {#each RDP_RESIZE as option (option.value)}<option value={option.value}>{option.label}</option>{/each}
        </Select>
      </div>
      <div class="cell span">
        <span class="label"><label for={inputId('rdp.security')}>Security</label><SaveMark mark={draft.marks['rdp.security']} /></span>
        <Select id={inputId('rdp.security')} bind:value={draft.values.rdp.security} onblur={() => void draft.commit('rdp.security')} onkeydown={(event) => { onSelectKey(event, 'rdp.security'); }}>
          {#each RDP_SECURITY as option (option.value)}<option value={option.value}>{option.label}</option>{/each}
        </Select>
      </div>
      <FieldErrors errors={draft.errorsIn('rdp')} />
    </div>
    <div class="flags">
      {#each RDP_FLAGS as flag (flag.key)}
        <span class="flag">
          <Switch labelledby="device-rdp-{flag.key}" bind:checked={draft.values.rdp[flag.key]} onchange={() => void draft.commit(`rdp.${flag.key}`)} />
          <span id="device-rdp-{flag.key}">{flag.label}</span><SaveMark mark={draft.marks[`rdp.${flag.key}`]} />
        </span>
      {/each}
    </div>
    {#each warnings as warning (warning)}
      <p class="warning" role="note">{warning}</p>
    {/each}
    {#if certificate !== null}
      <dl class="certificate" aria-label="Server certificate">
        <dt>Fingerprint (SHA-256)</dt>
        <dd>{certificate.fingerprintSha256}</dd>
        {#if certificate.subject}<dt>Subject</dt><dd>{certificate.subject}</dd>{/if}
        {#if certificate.notAfter}<dt>Valid until</dt><dd>{validUntil(certificate.notAfter)}</dd>{/if}
      </dl>
    {/if}
    {#if draft.values.rdp.enabled}
      <div class="foot">
        <RemoteTest
          protocol="RDP"
          initial={saved}
          values={draft.values}
          deviceId={device.id}
          canTest={() => draft.checkForTest('rdp')}
          acceptedFingerprint={certificate?.fingerprintSha256}
          {onTest}
          onAcceptServerCertificate={calls.onAcceptServerCertificate}
        />
      </div>
    {/if}
  </ConnectionRow>
</div>

<style>
  .fields {
    display: grid;
    grid-template-columns: calc(var(--space-16) * 1.75) minmax(0, 1fr) minmax(0, 1fr);
    gap: var(--space-4);
  }

  .cell {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-inline-size: 0;
  }

  .span {
    grid-column: span 2;
  }

  .cell > :global(.select) {
    inline-size: 100%;
  }

  .label {
    display: flex;
    align-items: baseline;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .flags {
    display: grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    gap: var(--space-3) var(--space-6);
    margin-block-start: var(--space-4);
  }

  .flag {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .warning {
    margin: var(--space-3) 0 0;
    color: var(--status-warning-text);
    font-size: var(--text-sm);
  }

  .certificate {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--space-1) var(--space-3);
    margin: var(--space-4) 0 0;
    font-size: var(--text-sm);
  }

  dt {
    color: var(--text-secondary);
  }

  dd {
    margin: 0;
    font-family: var(--font-mono);
    overflow-wrap: anywhere;
  }

  .foot {
    margin-block-start: var(--space-4);
  }

  @media (max-width: 767.98px) {
    .fields {
      grid-template-columns: calc(var(--space-16) * 1.5) minmax(0, 1fr);
    }

    .cell,
    .span {
      grid-column: 1 / -1;
    }

    .flags {
      grid-template-columns: minmax(0, 1fr);
    }
  }
</style>
