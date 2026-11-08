<script lang="ts">
  import type { HubError, HubResult } from '$lib/hub/hub';
  import FormButton from '../../../../forms/FormButton.svelte';
  import type { DeviceFormValues } from '../../values';
  import { validUntil } from '../warnings';
  import {
    isRemoteWarning,
    remoteOutcome,
    remoteOutcomeText,
    remoteTestInput,
    testsSavedRemote,
    type RemoteOutcome,
    type RemoteProtocol,
    type RemoteTestInput,
    type RemoteTestResult,
  } from './remote-test';

  interface Props {
    protocol: RemoteProtocol;
    initial: DeviceFormValues | null;
    values: DeviceFormValues;
    /** Null for a device that is not saved yet. */
    deviceId: string | null;
    /** Checks the form first; false means the test must not run. */
    canTest: () => boolean;
    /** The saved device's accepted certificate fingerprint (RDP): seeing it again is not news. */
    acceptedFingerprint?: string | null;
    onTest: (input: RemoteTestInput) => Promise<HubResult<RemoteTestResult>>;
    /** Only for a saved device. Without it the Accept button is hidden (D-4). */
    onAcceptServerCertificate?: (fingerprint: string) => Promise<HubResult<unknown>>;
  }

  const { protocol, initial, values, deviceId, canTest, acceptedFingerprint = null, onTest, onAcceptServerCertificate }: Props = $props();
  let testing = $state(false);
  let accepting = $state(false);
  let outcome = $state<RemoteOutcome | null>(null);
  let failure = $state<HubError | null>(null);
  let accepted = $state(false);
  /** What the shown result is about: the test input then, and whether it ran against the saved record. */
  let tested = $state<{ signature: string; saved: boolean } | null>(null);

  // The input is serialised only to compare it; the text is never shown or sent.
  const signature = $derived(JSON.stringify(remoteTestInput(protocol, initial, values, deviceId)));
  const fresh = $derived(tested !== null && tested.signature === signature);
  const shown = $derived(fresh ? outcome : null);
  const shownFailure = $derived(fresh ? failure : null);
  const certificate = $derived(shown?.kind === 'certificateNew' || shown?.kind === 'certificateChanged' ? shown.certificate : null);
  const acceptable = $derived(fresh && tested !== null && tested.saved && onAcceptServerCertificate !== undefined && certificate !== null);

  async function test(): Promise<void> {
    if (testing || !canTest()) return;
    testing = true;
    outcome = null;
    failure = null;
    accepted = false;
    const input = remoteTestInput(protocol, initial, values, deviceId);
    tested = { signature: JSON.stringify(input), saved: testsSavedRemote(protocol, initial, values, deviceId) };
    const result = await onTest(input);
    testing = false;
    if (result.ok) outcome = remoteOutcome(protocol, result.value, acceptedFingerprint);
    else failure = result.error;
  }

  async function accept(): Promise<void> {
    if (accepting || onAcceptServerCertificate === undefined || certificate === null) return;
    accepting = true;
    failure = null;
    const result = await onAcceptServerCertificate(certificate.fingerprintSha256);
    accepting = false;
    if (result.ok) {
      outcome = null;
      accepted = true;
    } else {
      failure = result.error;
    }
  }
</script>

<div class="test">
  <FormButton data-connection-test={protocol.toLowerCase()} onclick={test} aria-disabled={testing} aria-busy={testing}>
    Test connection
  </FormButton>
  <div role="status">
    {#if shown !== null}
      <p class:warning={isRemoteWarning(shown)}>{remoteOutcomeText(shown)}</p>
    {:else if accepted}
      <p>Certificate accepted. Test again to check the sign-in.</p>
    {/if}
  </div>
  {#if certificate !== null}
    <dl aria-label="Server certificate seen">
      <dt>Fingerprint (SHA-256)</dt>
      <dd>{certificate.fingerprintSha256}</dd>
      {#if certificate.subject}
        <dt>Subject</dt>
        <dd>{certificate.subject}</dd>
      {/if}
      {#if certificate.notAfter}
        <dt>Valid until</dt>
        <dd>{validUntil(certificate.notAfter)}</dd>
      {/if}
    </dl>
    {#if acceptable}
      <FormButton tone={shown?.kind === 'certificateChanged' ? 'danger' : 'primary'} onclick={accept} aria-disabled={accepting} aria-busy={accepting}>
        Accept certificate
      </FormButton>
    {:else if onAcceptServerCertificate !== undefined && tested?.saved !== true}
      <p class="note">Save the device, then test again to accept this certificate.</p>
    {/if}
  {/if}
  {#if shownFailure !== null}
    <p class="warning" role="alert">{shownFailure.message}</p>
  {/if}
</div>

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

  .warning {
    color: var(--status-down);
  }

  .note {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  dl {
    display: grid;
    grid-template-columns: max-content 1fr;
    gap: var(--space-1) var(--space-3);
    margin: 0;
    font-size: var(--text-sm);
  }

  dt {
    color: var(--text-muted);
  }

  dd {
    margin: 0;
    color: var(--text);
    font-family: var(--font-mono);
    overflow-wrap: anywhere;
  }
</style>
