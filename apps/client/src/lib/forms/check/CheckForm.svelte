<script lang="ts">
  import { tick, untrack } from 'svelte';
  import { fieldErrorsFrom } from '../device/field-errors';
  import type { FieldError, HubResult } from '$lib/hub/error';
  import { flip, measure } from '$lib/motion/roles/flip';
  import { crossFade } from '$lib/motion/roles/transitions';
  import AdvancedFields from './fields/AdvancedFields.svelte';
  import GroupFields from './fields/GroupFields.svelte';
  import HttpFields from './fields/HttpFields.svelte';
  import PressButton from './fields/PressButton.svelte';
  import SelectField from './fields/SelectField.svelte';
  import TextField from './fields/TextField.svelte';
  import TypeFields from './fields/TypeFields.svelte';
  import { messageFor, unplaced } from './model/errors';
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
  ];
  /** The field names this form has an input for; any other error is listed at the top. */
  const SHOWN = new Set([
    'name', 'intervalSeconds', 'retries', 'retryIntervalSeconds', 'degradedThresholdMs', 'timeoutMs', 'port',
    'serverName', 'url', 'maxRedirects', 'acceptedStatus', 'headers', 'body', 'keyword', 'jsonQuery',
    'jsonExpected', 'auth', 'username', 'password', 'bearerToken', 'dnsName', 'resolver', 'expected',
  ]);
  const COMMON = new Set(['intervalSeconds', 'retries', 'retryIntervalSeconds', 'degradedThresholdMs']);

  const initial = untrack(() => checkValues(check, 'ICMP'));
  let values = $state(initial);
  let errors = $state<FieldError[]>([]);
  let saving = $state(false);
  let advancedOpen = $state(false);
  let actions: HTMLElement | undefined = $state();

  const candidates = $derived(groupCandidates(check?.id ?? null, siblings));
  const warnings = $derived(check === null ? [] : secretWarnings(initial, values));
  const error = (field: string) => messageFor(errors, field);

  /** Disclosure (5.7.3): the content fades in or out, and what lies below moves by FLIP on `spring-default`. */
  async function toggleAdvanced(): Promise<void> {
    const first = measure(actions === undefined ? [] : [actions]);
    advancedOpen = !advancedOpen;
    await tick();
    flip(first, { spring: 'default' });
  }

  async function submit(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    if (saving) return;
    errors = validateCheck(values, check === null ? null : initial);
    if (errors.length > 0) {
      if (errors.some((found) => COMMON.has(found.field))) advancedOpen = true;
      return;
    }
    saving = true;
    const result = await onSave($state.snapshot(values));
    saving = false;
    if (!result.ok) errors = fieldErrorsFrom(result.error);
  }
</script>

<form onsubmit={submit} novalidate>
  {#if unplaced(errors, SHOWN).length > 0}
    <div class="alert" role="alert">
      {#each unplaced(errors, SHOWN) as found (found.field + found.message)}
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
    <GroupFields {candidates} bind:childIds={values.group.childIds} />
  {:else}
    <TypeFields bind:values {errors} />
  {/if}

  <section>
    <PressButton
      label={advancedOpen ? 'Hide advanced settings' : 'Advanced settings'}
      expanded={advancedOpen}
      onclick={toggleAdvanced}
    />
    {#if advancedOpen}
      <div class="advanced" transition:crossFade>
        <AdvancedFields bind:values {errors} />
      </div>
    {/if}
  </section>

  {#if warnings.length > 0}
    <div class="warning" role="status">
      {#each warnings as warning (warning)}
        <p>{warning}</p>
      {/each}
    </div>
  {/if}

  <div class="actions" bind:this={actions}>
    <PressButton label="Cancel" onclick={onCancel} />
    <PressButton label={check === null ? 'Add check' : 'Save check'} kind="primary" submit disabled={saving} />
  </div>
</form>

<style>
  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    max-inline-size: calc(var(--space-16) * 8);
  }

  .advanced {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding-block-start: var(--space-3);
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
