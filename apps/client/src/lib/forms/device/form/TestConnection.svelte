<script lang="ts">
  import type { HubError, HubResult } from '$lib/hub/error';
  import FormButton from '../../FormButton.svelte';
  import HostKeyModal from '../../modal/HostKeyModal.svelte';
  import { connectionTestInput, testOutcome, testsStoredCredential, type TestOutcome } from '../connection-test';
  import type { ConnectionTestInput, ConnectionTestResult } from '../hub-types';
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

  const saved = $derived(deviceId !== null);
  const advice = $derived(saved && !testsStoredCredential(initial, values, deviceId) && credentialInput(values) === null);

  async function test(): Promise<void> {
    if (testing || !canTest()) return;
    testing = true;
    outcome = null;
    failure = null;
    const result = await onTest(connectionTestInput(initial, values, deviceId));
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
  <FormButton onclick={test} disabled={testing} aria-busy={testing}>Test connection</FormButton>
  {#if advice}
    <p class="advice">
      This test does not use the stored credential, because the host or the SSH settings changed. Enter the credential to
      test it.
    </p>
  {/if}
  <div role="status">
    {#if outcome !== null}
      <p class:warning={isWarning(outcome)}>{outcomeText(outcome)}</p>
      {#if outcome.kind === 'hostKeyNew' || outcome.kind === 'hostKeyChanged'}
        <FormButton onclick={() => { keyOpen = true; }}>Show host key</FormButton>
      {/if}
    {/if}
  </div>
  {#if failure !== null}
    <p class="warning" role="alert">{failure.message}</p>
  {/if}
</div>
{#if outcome !== null && (outcome.kind === 'hostKeyNew' || outcome.kind === 'hostKeyChanged')}
  <HostKeyModal
    open={keyOpen}
    hostKey={outcome.hostKey}
    changed={outcome.kind === 'hostKeyChanged'}
    {saved}
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
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  .warning {
    color: var(--status-down);
  }
</style>
