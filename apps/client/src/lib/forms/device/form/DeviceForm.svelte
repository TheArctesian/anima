<script lang="ts">
  import { tick, untrack } from 'svelte';
  import type { FieldError, HubResult } from '$lib/hub/error';
  import FormButton from '../../FormButton.svelte';
  import { fieldErrorsFrom } from '../field-errors';
  import type { ConnectionTestInput } from '$lib/hub/generated/graphql';
  import type { ConnectionTestResult } from '../connection-test';
  import { validateDevice } from '../validate';
  import { formValues, type DeviceFields, type DeviceFormValues, type SiteOption } from '../values';
  import type { EditedValues } from './editing';
  import DisclosureSection from './DisclosureSection.svelte';
  import { firstInvalidField, formErrors, inputId, replaceField } from './errors';
  import GeneralSection from './GeneralSection.svelte';
  import SshSection from './SshSection.svelte';
  import TestConnection from './TestConnection.svelte';

  interface Props {
    /** The saved device, or null to create one. */
    device: DeviceFields | null;
    /** Null for a device that is not saved yet. */
    deviceId?: string | null;
    /** K1 `credentialStored` of the saved device. */
    credentialStored?: boolean;
    sites?: readonly SiteOption[];
    /** The section to open and bring into view (DEV-022). */
    section?: 'general' | 'ssh' | 'rdp';
    /** `siteId` is null when editing. The values are a snapshot with secrets in it: the owner must not keep it. */
    onSave: (values: DeviceFormValues, siteId: string | null) => Promise<HubResult<unknown>>;
    onTest: (input: ConnectionTestInput) => Promise<HubResult<ConnectionTestResult>>;
    /** Only for a saved device. */
    onAcceptHostKey: (fingerprint: string) => Promise<HubResult<unknown>>;
    onClearCredential: () => Promise<HubResult<unknown>>;
    onCancel: () => void;
  }

  const {
    device,
    deviceId = null,
    credentialStored = false,
    sites = [],
    section,
    onSave,
    onTest,
    onAcceptHostKey,
    onClearCredential,
    onCancel,
  }: Props = $props();

  const initial = untrack(() => formValues(device));
  let values = $state<EditedValues>(structuredClone(initial) as EditedValues);
  let siteId = $state(untrack(() => sites[0]?.id ?? ''));
  let errors = $state<FieldError[]>([]);
  let saving = $state(false);
  /** Counts successful saves; the connection test starts afresh after each, so no typed secret outlives the save. */
  let saves = $state(0);
  let clearing = $state(false);
  let general = $state(true);
  let ssh = $state(untrack(() => section !== 'rdp'));
  let rdp = $state(untrack(() => section === 'rdp'));
  let root = $state<HTMLElement>();

  const creating = $derived(device === null);

  $effect(() => {
    const target = section === undefined ? null : (root?.querySelector<HTMLElement>(`[data-section="${section}"] button`) ?? null);
    if (target === null) return;
    target.focus();
    if (typeof target.scrollIntoView === 'function') target.scrollIntoView({ block: 'nearest' });
  });

  async function show(found: readonly FieldError[]): Promise<void> {
    errors = [...found];
    const field = firstInvalidField(found);
    if (field === undefined) return;
    if (field.startsWith('ssh.') || field.startsWith('credential.')) ssh = true;
    await tick();
    root?.querySelector<HTMLElement>(`#${inputId(field)}`)?.focus();
  }

  function checkHost(): void {
    errors = replaceField(errors, 'host', validateDevice(values));
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
    if (creating && siteId === '') found.push({ field: 'siteId', message: 'Add a site before you add a device.' });
    if (found.length > 0) {
      await show(found);
      return;
    }
    saving = true;
    const result = await onSave($state.snapshot(values), creating ? siteId : null);
    saving = false;
    if (result.ok) {
      values.credential = { password: '', privateKey: '', passphrase: '' };
      errors = [];
      saves += 1;
    } else {
      await show(fieldErrorsFrom(result.error));
    }
  }

  async function clearCredential(): Promise<void> {
    if (clearing) return;
    clearing = true;
    const result = await onClearCredential();
    clearing = false;
    if (!result.ok) errors = fieldErrorsFrom(result.error);
  }
</script>

<div bind:this={root}>
  <form
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
      <DisclosureSection title="General" bind:open={general}>
        <GeneralSection bind:values {errors} sites={creating ? sites : []} bind:siteId onHostBlur={checkHost} />
      </DisclosureSection>
    </div>
    <div data-section="ssh">
      <DisclosureSection title="SSH" bind:open={ssh}>
        <SshSection bind:values {errors} {credentialStored} {clearing} onClearCredential={clearCredential} />
        {#if values.ssh.enabled}
          {#key saves}
          <TestConnection {initial} {values} {deviceId} {canTest} {onTest} {onAcceptHostKey} />
        {/key}
        {/if}
      </DisclosureSection>
    </div>
    <div data-section="rdp">
      <DisclosureSection title="Remote Desktop (RDP)" bind:open={rdp}>
        <p>RDP comes in a later version of Anima.</p>
      </DisclosureSection>
    </div>
    <div class="actions">
      <FormButton onclick={onCancel}>Cancel</FormButton>
      <FormButton type="submit" tone="primary" aria-disabled={saving} aria-busy={saving}>Save</FormButton>
    </div>
  </form>
</div>

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
    padding-top: var(--space-3);
  }
</style>
