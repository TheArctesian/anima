<script lang="ts">
  import type { HubError, HubResult } from '$lib/hub/hub';
  import FormButton from '../../../forms/FormButton.svelte';
  import HostKeyModal from '../../../forms/modal/HostKeyModal.svelte';
  import {
    connectionTestInput,
    testOutcome,
    testsSavedRecord,
    testsStoredCredential,
    type ConnectionTestResult,
    type TestOutcome,
  } from '../connection-test';
  import type { ConnectionTestInput } from '$lib/hub/generated/graphql';
  import { credentialInput } from '../input';
  import type { DeviceFormValues } from '../values';
  import { isWarning, outcomeText } from './errors';

  interface Props {
    initial: DeviceFormValues | null;
    values: DeviceFormValues;
    /** Null for a device that is not saved yet. */
    deviceId: string | null;
    /** Checks the host and SSH fields first; false means the test must not run. */
    canTest: () => boolean;
    onTest: (input: ConnectionTestInput) => Promise<HubResult<ConnectionTestResult>>;
    onAcceptHostKey: (fingerprint: string) => Promise<HubResult<unknown>>;
  }

  const { initial, values, deviceId, canTest, onTest, onAcceptHostKey }: Props = $props();
  let testing = $state(false);
  let outcome = $state<TestOutcome | null>(null);
  let failure = $state<HubError | null>(null);
  let keyOpen = $state(false);
  /** What the shown result is about: the test input then, and whether it ran against the saved record. */
  let tested = $state<{ signature: string; saved: boolean } | null>(null);

  // The input is serialised only to compare it; the text is never shown or sent.
  const signature = $derived(JSON.stringify(connectionTestInput(initial, values, deviceId)));
  const fresh = $derived(tested !== null && tested.signature === signature);
  const shown = $derived(fresh ? outcome : null);
  const shownFailure = $derived(fresh ? failure : null);
  const acceptable = $derived(fresh && tested !== null && tested.saved);
  const advice = $derived(deviceId !== null && !testsStoredCredential(initial, values, deviceId) && credentialInput(values) === null);

  async function test(): Promise<void> {
    if (testing || !canTest()) return;
    testing = true;
    outcome = null;
    failure = null;
    const input = connectionTestInput(initial, values, deviceId);
    tested = { signature: JSON.stringify(input), saved: testsSavedRecord(initial, values, deviceId) };
    const result = await onTest(input);
    testing = false;
    if (!result.ok) {
      failure = result.error;
      return;
    }
    outcome = testOutcome(result.value);
    keyOpen = outcome.kind === 'hostKeyNew' || outcome.kind === 'hostKeyChanged';
  }

  async function accept(fingerprint: string): Promise<HubResult<unknown>> {
    const result = await onAcceptHostKey(fingerprint);
    if (result.ok) outcome = null;
    return result;
  }
</script>

<div class="test">
  <FormButton data-connection-test onclick={test} aria-disabled={testing} aria-busy={testing}>Test connection</FormButton>
  {#if advice}
    <p class="advice">
      This test does not use the stored credential, because the host or the SSH settings changed. Enter the credential to
      test it.
    </p>
  {/if}
  <div role="status">
    {#if shown !== null}
      <p class:warning={isWarning(shown)}>{outcomeText(shown)}</p>
      {#if shown.kind === 'hostKeyNew' || shown.kind === 'hostKeyChanged'}
        <FormButton onclick={() => { keyOpen = true; }}>Show host key</FormButton>
      {/if}
    {/if}
  </div>
  {#if shownFailure !== null}
    <p class="warning" role="alert">{shownFailure.message}</p>
  {/if}
</div>
{#if shown !== null && (shown.kind === 'hostKeyNew' || shown.kind === 'hostKeyChanged')}
  <HostKeyModal
    open={keyOpen}
    hostKey={shown.hostKey}
    changed={shown.kind === 'hostKeyChanged'}
    saved={acceptable}
    onAccept={accept}
    onClose={() => { keyOpen = false; }}
  />
{/if}

<style>
  .test {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
  }

  p {
    margin: 0;
  }

  .advice {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .warning {
    color: var(--status-down);
  }
</style>
