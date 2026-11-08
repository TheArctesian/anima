<script lang="ts">
  import { untrack } from 'svelte';
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
  import SnmpFields from './probes/SnmpFields.svelte';
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
    // DOMAIN_EXPIRY is not offered until its editor exists: WP-50 (D-29).
  ];

  // Inside a modal the actions go to its footer; the submit button is tied to the form by its ID.
  const footer = modalFooter();
  const formId = $props.id();
  $effect.pre(() => footer?.show(actions));

  const initial = untrack(() => checkValues(check, 'ICMP'));
  let values = $state(initial);
  let errors = $state<FieldError[]>([]);
  let saving = $state(false);
  let advancedOpen = $state(false);

  const candidates = $derived(groupCandidates(check?.id ?? null, siblings));
  const warnings = $derived(check === null ? [] : secretWarnings(initial, values));
  const alerts = $derived(unplaced(errors, shownFields(values)));
  const error = (field: string) => messageFor(errors, field);

  /** Shows `found`, and opens Advanced when one of its settings is wrong, so that no error stays hidden. */
  function show(found: FieldError[]): void {
    errors = found;
    if (touchesAdvanced(found)) advancedOpen = true;
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (saving) return;
    const invalid = validateCheck(values, check === null ? null : initial);
    show(invalid);
    if (invalid.length > 0) return;
    saving = true;
    try {
      const result = await onSave($state.snapshot(values));
      if (!result.ok) show(placeHubErrors(fieldErrorsFrom(result.error), values));
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
  {:else if values.type === 'SNMP'}
    <SnmpFields bind:snmp={values.snmp} {errors} />
  {:else if values.type === 'DOCKER' || values.type === 'SYSTEMD' || values.type === 'TAILSCALE_PING'}
    <ProbeFields bind:values {errors} />
  {:else}
    <TypeFields bind:values {errors} />
  {/if}

  <Disclosure label="Advanced settings" bind:open={advancedOpen}>
    <AdvancedFields bind:values {errors} />
  </Disclosure>

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
    <PressButton label={check === null ? 'Add check' : 'Save check'} kind="primary" submit form={formId} disabled={saving} />
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
