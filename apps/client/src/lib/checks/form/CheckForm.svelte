<script lang="ts">
  import { tick, untrack } from 'svelte';
  import { SvelteSet } from 'svelte/reactivity';
  import { fieldErrorsFrom } from '$lib/devices/form/field-errors';
  import Disclosure from '$lib/forms/disclosure/Disclosure.svelte';
  import { modalFooter } from '$lib/forms/modal/footer';
  import type { FieldError, HubResult } from '$lib/hub/hub';
  import AdvancedFields from './fields/AdvancedFields.svelte';
  import GroupFields from './fields/GroupFields.svelte';
  import HttpFields from './fields/HttpFields.svelte';
  import PressButton from './fields/PressButton.svelte';
  import SelectField from './fields/SelectField.svelte';
  import TextField from './fields/TextField.svelte';
  import TypeFields from './fields/TypeFields.svelte';
  import ProbeFields from './probes/ProbeFields.svelte';
  import DomainExpiryFields from './probes/DomainExpiryFields.svelte';
  import { contextChannelCalls } from './channels/calls';
  import { ChannelsDraft } from './channels/draft.svelte';
  import ChannelsSection from './channels/ChannelsSection.svelte';
  import { intervalFor } from './probes/domain-expiry';
  import SnmpFields from './probes/SnmpFields.svelte';
  import { defaultsReader } from '../defaults/reader';
  import { untouchedSettings, type SettingKey } from '../defaults/settings';
  import { messageFor, placeHubErrors, shownFields, touchesAdvanced, unplaced } from './model/errors';
  import { groupCandidates, type SiblingCheck } from './model/group';
  import type { CheckFields, CheckType } from './model/hub-types';
  import { validateCheck } from './model/validate';
  import { secretWarnings } from './model/warnings';
  import { checkValues, type CheckFormValues } from './model/values';

  interface Props {
    /** The check to edit, or null to create one. */
    check: CheckFields | null;
    /** The device's other checks; a group chooses its children among them. */
    siblings: readonly SiblingCheck[];
    /** Saves the values; the route builds `createCheckInput` or `updateCheckInput` from them. */
    onSave: (values: CheckFormValues) => Promise<HubResult<unknown>>;
    onCancel: () => void;
  }

  const { check, siblings, onSave, onCancel }: Props = $props();

  const TYPES: readonly { value: CheckType; label: string }[] = [
    { value: 'ICMP', label: 'Ping (ICMP)' },
    { value: 'TCP', label: 'TCP port' },
    { value: 'TLS', label: 'TLS certificate' },
    { value: 'HTTP', label: 'HTTP or HTTPS' },
    { value: 'DNS', label: 'DNS lookup' },
    { value: 'PUSH', label: 'Push (the device calls the hub)' },
    { value: 'GROUP', label: 'Group of checks' },
    { value: 'DOCKER', label: 'Docker container' },
    { value: 'SYSTEMD', label: 'systemd service' },
    { value: 'SNMP', label: 'SNMP' },
    { value: 'TAILSCALE_PING', label: 'Tailscale ping' },
    { value: 'DOMAIN_EXPIRY', label: 'Domain expiry' },
  ];

  // Inside a modal the actions go to its footer; the submit button is tied to the form by its ID.
  const footer = modalFooter();
  const formId = $props.id();
  $effect.pre(() => footer?.show(actions));

  const initial = untrack(() => checkValues(check, 'ICMP'));
  let values = $state(initial);
  // D-26: a new check starts from the K1 input defaults at once and takes the hub's defaults when they arrive, so a
  // slow or failed answer never holds the form. A setting the operator has touched is left as it is.
  const touched = new SvelteSet<SettingKey>();
  /** The interval a new check of any other type than domain expiry shows: the K1 default, then the hub's. */
  let ordinaryInterval = untrack(() => initial.intervalSeconds);
  const readDefaults = untrack(() => (check === null ? defaultsReader() : null));
  $effect(() => {
    void readDefaults?.().then((defaults) => {
      if (defaults === null) return;
      const patch = untouchedSettings(touched, defaults);
      if (patch.intervalSeconds !== undefined) {
        ordinaryInterval = patch.intervalSeconds;
        patch.intervalSeconds = intervalFor(values.type, values.intervalSeconds, ordinaryInterval, false);
      }
      Object.assign(values, patch);
    });
  });
  // D-29: a new domain expiry check proposes 12 hours; any other type takes the ordinary interval back. A touched one stays.
  $effect(() => {
    const { type } = values;
    untrack(() => {
      if (check === null) values.intervalSeconds = intervalFor(type, values.intervalSeconds, ordinaryInterval, touched.has('intervalSeconds'));
    });
  });
  let errors = $state<FieldError[]>([]);
  let saving = $state(false);
  let advancedOpen = $state(false);
  /** NOT-003, NOT-005: a check reads and saves its own channels apart from the check (F-3); null without a hub. */
  const channels = new ChannelsDraft(untrack(() => check?.id ?? null), untrack(() => contextChannelCalls()));
  const resendId = `${formId}-resend`;

  const candidates = $derived(groupCandidates(check?.id ?? null, siblings));
  const warnings = $derived(check === null ? [] : secretWarnings(initial, values));
  const alerts = $derived(unplaced(errors, shownFields(values)));
  const error = (field: string) => messageFor(errors, field);

  /** Shows `found`, and opens Advanced when one of its settings is wrong, so that no error stays hidden. */
  function show(found: FieldError[]): void {
    errors = found;
    if (touchesAdvanced(found)) advancedOpen = true;
  }

  /** The ID of a check the route has just created, read from its answer. */
  function createdId(value: unknown): string | null {
    if (typeof value !== 'object' || value === null || !('id' in value)) return null;
    return typeof value.id === 'string' ? value.id : null;
  }

  async function focusResend(): Promise<void> {
    await tick();
    document.getElementById(resendId)?.focus();
  }

  /**
   * An edit saves the channels first: the route closes the form once the check is saved, and a refusal of the
   * channels must still be seen. A new check has no ID yet, so its channels go in after it.
   */
  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (saving) return;
    const invalid = validateCheck(values, check === null ? null : initial);
    show(invalid);
    if (invalid.length > 0) return;
    if (!channels.check()) {
      await focusResend();
      return;
    }
    saving = true;
    try {
      const sent = check === null ? 'skipped' : await channels.save(check.id);
      if (sent === 'failed') {
        await focusResend();
        return;
      }
      const result = await onSave($state.snapshot(values));
      if (!result.ok) {
        channels.checkNotSaved();
        show(placeHubErrors(fieldErrorsFrom(result.error), values));
        return;
      }
      const id = check === null ? createdId(result.value) : null;
      if (id !== null && (await channels.save(id)) === 'failed') await focusResend();
    } finally {
      saving = false;
    }
  }
</script>

<form id={formId} onsubmit={submit} novalidate>
  {#if alerts.length > 0}
    <div class="alert" role="alert">
      {#each alerts as found, index (index)}
        <p>{found.message}</p>
      {/each}
    </div>
  {/if}

  <TextField label="Name" bind:value={values.name} error={error('name')} />

  {#if check === null}
    <SelectField label="Type" bind:value={values.type} options={TYPES} />
  {:else}
    <!-- CHK-009: the type is fixed once the check exists. -->
    <p class="type">Type: {TYPES.find((entry) => entry.value === values.type)?.label}</p>
  {/if}

  {#if values.type === 'HTTP'}
    <HttpFields bind:http={values.http} {errors} />
  {:else if values.type === 'GROUP'}
    <GroupFields {candidates} bind:childIds={values.group.childIds} error={error('childIds')} />
  {:else if values.type === 'DOMAIN_EXPIRY'}
    <DomainExpiryFields bind:domainExpiry={values.domainExpiry} {errors} />
  {:else if values.type === 'SNMP'}
    <SnmpFields bind:snmp={values.snmp} {errors} />
  {:else if values.type === 'DOCKER' || values.type === 'SYSTEMD' || values.type === 'TAILSCALE_PING'}
    <ProbeFields bind:values {errors} />
  {:else}
    <TypeFields bind:values {errors} />
  {/if}

  <Disclosure label="Advanced settings" bind:open={advancedOpen}>
    <AdvancedFields bind:values {errors} ontouch={(key) => touched.add(key)} />
  </Disclosure>

  {#if channels.phase !== 'absent'}
    <ChannelsSection draft={channels} {resendId} />
  {/if}

  {#if warnings.length > 0}
    <div class="warning" role="status">
      {#each warnings as warning (warning)}
        <p>{warning}</p>
      {/each}
    </div>
  {/if}

  {#if footer === undefined}
    {@render actions()}
  {/if}
</form>

{#snippet actions()}
  <div class="actions">
    <PressButton label="Cancel" onclick={onCancel} />
    <PressButton label={check === null ? 'Add check' : 'Save check'} kind="primary" submit form={formId} busy={saving} />
  </div>
{/snippet}

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    max-inline-size: calc(var(--space-16) * 8);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .type {
    margin: 0;
    color: var(--text-muted);
  }

  .alert,
  .warning {
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    font-size: var(--text-sm);
  }

  .alert {
    background: var(--status-down-tint);
  }

  .warning {
    background: var(--status-warning-tint);
  }

  p {
    margin: 0;
  }
</style>
