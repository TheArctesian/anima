<script lang="ts">
  import type { HubError, HubResult } from '$lib/hub/hub';
  import FormButton from '../FormButton.svelte';
  import type { HostKey } from '../../devices/form/values';
  import Modal from './Modal.svelte';

  interface Props {
    open: boolean;
    hostKey: HostKey;
    /** True when the key differs from the pinned one (SEC-007). */
    changed: boolean;
    /** True when the test ran against the saved record: the hub pins a key to a saved device only (D-5). */
    saved: boolean;
    onAccept: (fingerprint: string) => Promise<HubResult<unknown>>;
    onClose: () => void;
  }

  const { open, hostKey, changed, saved, onAccept, onClose }: Props = $props();
  let busy = $state(false);
  let failure = $state<HubError | null>(null);

  async function accept(): Promise<void> {
    if (busy) return;
    busy = true;
    failure = null;
    const result = await onAccept(hostKey.fingerprintSha256);
    busy = false;
    if (result.ok) onClose();
    else failure = result.error;
  }
</script>

<Modal {open} title={changed ? 'Host key changed' : 'New host key'} {onClose}>
  {#if changed}
    <p class="warning" role="alert">The host key changed. Do not trust this device until you check it.</p>
  {:else}
    <p>This device has no recorded host key yet. Check that the fingerprint is the one you expect.</p>
  {/if}
  <dl>
    <dt>Algorithm</dt>
    <dd>{hostKey.algorithm}</dd>
    <dt>SHA-256 fingerprint</dt>
    <dd class="fingerprint">{hostKey.fingerprintSha256}</dd>
    <dt>First seen</dt>
    <dd><time datetime={hostKey.firstSeenAt}>{hostKey.firstSeenAt}</time></dd>
  </dl>
  {#if !saved}
    <p>Save the device, then test again to accept this key.</p>
  {/if}
  {#if failure !== null}
    <p class="warning" role="alert">{failure.message}</p>
  {/if}
  <div class="actions">
    <FormButton data-autofocus onclick={onClose}>Close</FormButton>
    {#if saved}
      <FormButton tone={changed ? 'danger' : 'primary'} onclick={accept} aria-disabled={busy} aria-busy={busy}>
        Accept
      </FormButton>
    {/if}
  </div>
</Modal>

<style>
  p {
    margin: 0;
    line-height: var(--leading-normal);
  }

  .warning {
    padding: var(--space-3);
    background: var(--status-down-tint);
    border-left: var(--border-focus) solid var(--status-down);
    border-radius: var(--radius-md);
    color: var(--status-down);
  }

  dl {
    display: grid;
    gap: var(--space-1);
    margin: 0;
  }

  dt {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  dd {
    margin: 0;
  }

  .fingerprint {
    overflow-wrap: anywhere;
    font-family: var(--font-mono);
  }

  .actions {
    display: flex;
    justify-content: flex-end;
    gap: var(--space-2);
  }
</style>
