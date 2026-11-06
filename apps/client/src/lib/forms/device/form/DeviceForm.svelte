<script lang="ts">
  import RemoteLoginButton from '$lib/desktop/RemoteLoginButton.svelte';
  import { tick, untrack } from 'svelte';
  import type { FieldError, HubResult } from '$lib/hub/error';
  import Disclosure from '../../disclosure/Disclosure.svelte';
  import FormButton from '../../FormButton.svelte';
  import { modalFooter } from '../../modal/footer';
  import { fieldErrorsFrom } from '../field-errors';
  import type { ConnectionTestInput } from '$lib/hub/generated/graphql';
  import type { ConnectionTestResult } from '../connection-test';
  import { suggestedSite, validateNewSite, type NewSite, type SitePlacement } from '../site';
  import { validateDevice } from '../validate';
  import { formValues, type DeviceFields, type DeviceFormValues, type SiteOption } from '../values';
  import type { EditedValues } from './editing';
  import { firstInvalidField, formErrors, inputId, replaceField, sectionOf } from './errors';
  import GeneralSection from './GeneralSection.svelte';
  import { emptyRemoteSecrets } from '../remote/values';
  import RdpSection from '../remote/RdpSection.svelte';
  import TransferSection from '../remote/TransferSection.svelte';
  import VncSection from '../remote/VncSection.svelte';
  import { vncWarning } from '../remote/warnings';
  import SshSection from './SshSection.svelte';
  import TestConnection from './TestConnection.svelte';

  interface Props {
    /** The saved device, or null to create one. */
    device: DeviceFields | null;
    /** Create mode: the values to start from in place of the empty form, such as this computer's (R-060). */
    initial?: DeviceFormValues;
    /** This Mac is being added and SSH did not answer on it, so the Remote Login button says why SSH is off. */
    remoteLoginOff?: boolean;
    /** Null for a device that is not saved yet. */
    deviceId?: string | null;
    /** K1 `credentialStored` of the saved device. */
    credentialStored?: boolean;
    /** K1 `rdp.credentialStored` and `vnc.credentialStored` of the saved device. */
    rdpCredentialStored?: boolean;
    vncCredentialStored?: boolean;
    sites?: readonly SiteOption[];
    /**
     * The section to open and bring into view (DEV-022), whenever it changes; `test` is the SSH section's
     * Test connection button (D-8: SSH on without an accepted host key).
     */
    section?: 'general' | 'ssh' | 'rdp' | 'vnc' | 'test';
    /** `site` is null when editing. The values are a snapshot with secrets in it: the owner must not keep it. */
    onSave: (values: DeviceFormValues, site: SitePlacement | null) => Promise<HubResult<unknown>>;
    onTest: (input: ConnectionTestInput) => Promise<HubResult<ConnectionTestResult>>;
    /** Only for a saved device. */
    onAcceptHostKey: (fingerprint: string) => Promise<HubResult<unknown>>;
    onClearCredential: () => Promise<HubResult<unknown>>;
    /** Without these the RDP and VNC Remove buttons are hidden (D-4). */
    onClearRdpCredential?: () => Promise<HubResult<unknown>>;
    onClearVncCredential?: () => Promise<HubResult<unknown>>;
    onCancel: () => void;
  }

  const {
    device,
    initial: start,
    remoteLoginOff = false,
    deviceId = null,
    credentialStored = false,
    rdpCredentialStored = false,
    vncCredentialStored = false,
    sites = [],
    section,
    onSave,
    onTest,
    onAcceptHostKey,
    onClearCredential,
    onClearRdpCredential,
    onClearVncCredential,
    onCancel,
  }: Props = $props();

  // Inside a modal the actions go to its footer; the Save button is tied to the form by its ID.
  const footer = modalFooter();
  const formId = $props.id();
  $effect.pre(() => footer?.show(actions));

  const initial = untrack(() => (device === null && start !== undefined ? start : formValues(device)));
  let values = $state<EditedValues>(structuredClone(initial) as EditedValues);
  let siteId = $state(untrack(() => sites[0]?.id ?? ''));
  /** The site to create with the device when the hub has none yet, offered from the host (VPN-002). */
  let newSite = $state<NewSite | null>(
    untrack(() => (device === null && sites.length === 0 ? suggestedSite(start?.host ?? '') : null)),
  );
  /** The last offer, so a new host replaces the site only while the operator has not edited it. */
  let offered = untrack(() => (newSite === null ? null : { ...newSite }));
  let errors = $state<FieldError[]>([]);
  let saving = $state(false);
  /** Counts successful saves; the connection test starts afresh after each, so no typed secret outlives the save. */
  let saves = $state(0);
  let clearing = $state(false);
  let general = $state(true);
  let ssh = $state(untrack(() => section !== 'rdp'));
  let rdp = $state(untrack(() => section === 'rdp'));
  let vnc = $state(untrack(() => section === 'vnc'));
  let files = $state(false);
  let root = $state<HTMLElement>();

  const creating = $derived(device === null);

  /** The control a `section` names: the section's disclosure button, or Test connection. */
  const SECTION_TARGETS = {
    general: '[data-section="general"] button',
    ssh: '[data-section="ssh"] button',
    rdp: '[data-section="rdp"] button',
    vnc: '[data-section="vnc"] button',
    test: '[data-section="ssh"] [data-connection-test]',
  } as const;

  /** Each new `section` opens its disclosure, then focuses and scrolls to its control (DEV-022, D-8). */
  $effect(() => {
    const wanted = section;
    if (wanted === undefined) return;
    untrack(() => {
      if (wanted === 'general') general = true;
      else if (wanted === 'rdp') rdp = true;
      else if (wanted === 'vnc') vnc = true;
      else ssh = true;
    });
    const reach = () => {
      const target = root?.querySelector<HTMLElement>(SECTION_TARGETS[wanted]) ?? null;
      if (target === null) return;
      target.focus();
      if (typeof target.scrollIntoView === 'function') target.scrollIntoView({ block: 'nearest' });
    };
    // At once for a control already shown; again once a section that was closed has opened.
    reach();
    void tick().then(reach);
  });

  async function show(found: readonly FieldError[]): Promise<void> {
    errors = [...found];
    const field = firstInvalidField(found);
    if (field === undefined) {
      // An error that belongs to no input is shown above the sections, which may be scrolled away.
      await tick();
      const alert = root?.querySelector<HTMLElement>('.alert');
      if (typeof alert?.scrollIntoView === 'function') alert.scrollIntoView({ block: 'nearest' });
      return;
    }
    openSection(sectionOf(field));
    await tick();
    root?.querySelector<HTMLElement>(`#${inputId(field)}`)?.focus();
  }

  function openSection(name: 'ssh' | 'rdp' | 'vnc' | undefined): void {
    if (name === 'ssh') ssh = true;
    else if (name === 'rdp') rdp = true;
    else if (name === 'vnc') vnc = true;
  }

  function checkHost(): void {
    errors = replaceField(errors, 'host', validateDevice(values));
    if (newSite === null || offered === null) return;
    if (newSite.name !== offered.name || newSite.subnet !== offered.subnet) return;
    newSite = suggestedSite(values.host);
    offered = { ...newSite };
  }

  /** Where the device goes: null when editing, else the chosen site or the one to create. */
  function placement(): SitePlacement | null {
    if (!creating) return null;
    return newSite === null ? { kind: 'existing', id: siteId } : { kind: 'new', site: { ...newSite } };
  }

  /** The test needs a valid host and valid SSH settings, not a name (D-3). */
  function canTest(): boolean {
    const found = validateDevice(values).filter((error) => error.field !== 'name');
    if (found.length > 0) void show(found);
    return found.length === 0;
  }

  async function save(): Promise<void> {
    if (saving) return;
    const found = validateDevice(values);
    if (newSite !== null) found.push(...validateNewSite(newSite));
    if (found.length > 0) {
      await show(found);
      return;
    }
    saving = true;
    const result = await onSave($state.snapshot(values), placement());
    saving = false;
    if (result.ok) {
      values.credential = { password: '', privateKey: '', passphrase: '' };
      values.remoteSecrets = emptyRemoteSecrets();
      errors = [];
      saves += 1;
    } else {
      await show(fieldErrorsFrom(result.error));
    }
  }

  /** Runs a remove-credential call at once (D-6); the error, if any, is shown. */
  async function clearWith(remove: () => Promise<HubResult<unknown>>): Promise<void> {
    if (clearing) return;
    clearing = true;
    const result = await remove();
    clearing = false;
    if (!result.ok) errors = fieldErrorsFrom(result.error);
  }
</script>

<div bind:this={root}>
  <form
    id={formId}
    novalidate
    onsubmit={(event) => {
      event.preventDefault();
      void save();
    }}
  >
    <div class="alert" role="alert">
      {#each formErrors(errors) as error, index (index)}
        <p>{error.message}</p>
      {/each}
    </div>
    <div data-section="general">
      <Disclosure label="General" level={3} bind:open={general}>
        <GeneralSection bind:values {errors} sites={creating ? sites : []} bind:siteId bind:newSite onHostBlur={checkHost} />
      </Disclosure>
    </div>
    <div data-section="ssh">
      <Disclosure label="SSH" level={3} bind:open={ssh}>
        <SshSection bind:values {errors} {credentialStored} {clearing} onClearCredential={() => void clearWith(onClearCredential)} />
        <RemoteLoginButton sshOff={remoteLoginOff} />
        {#if values.ssh.enabled}
          {#key saves}
          <TestConnection {initial} {values} {deviceId} {canTest} {onTest} {onAcceptHostKey} />
        {/key}
        {/if}
      </Disclosure>
    </div>
    <div data-section="rdp">
      <Disclosure label="Remote Desktop (RDP)" level={3} bind:open={rdp}>
        <RdpSection
          bind:values
          {errors}
          credentialStored={rdpCredentialStored}
          serverCertificate={device?.rdp?.serverCertificate ?? null}
          {clearing}
          onClearCredential={onClearRdpCredential && (() => void clearWith(onClearRdpCredential))}
        />
      </Disclosure>
    </div>
    <div data-section="vnc">
      <Disclosure label="VNC" level={3} bind:open={vnc}>
        <VncSection
          bind:values
          {errors}
          credentialStored={vncCredentialStored}
          warning={vncWarning(device?.accessWarnings ?? [])}
          {clearing}
          onClearCredential={onClearVncCredential && (() => void clearWith(onClearVncCredential))}
        />
      </Disclosure>
    </div>
    <div data-section="files">
      <Disclosure label="Files and clipboard" level={3} bind:open={files}>
        <TransferSection bind:values />
      </Disclosure>
    </div>
    {#if footer === undefined}
      {@render actions()}
    {/if}
  </form>
</div>

{#snippet actions()}
  <div class="actions" class:inline={footer === undefined}>
    <FormButton onclick={onCancel}>Cancel</FormButton>
    <FormButton type="submit" form={formId} tone="primary" aria-disabled={saving} aria-busy={saving}>Save</FormButton>
  </div>
{/snippet}

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .alert p {
    margin: 0;
    padding: var(--space-2) var(--space-3);
    background: var(--status-down-tint);
    border-radius: var(--radius-md);
    color: var(--status-down);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  /* In the form, after the sections; a modal's footer has its own space above. */
  .inline {
    padding-top: var(--space-3);
  }
</style>
