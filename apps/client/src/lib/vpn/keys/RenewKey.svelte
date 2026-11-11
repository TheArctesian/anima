<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import type { HubError, HubResult } from '$lib/hub/hub';
  import type { VpnNode } from '../api';
  import type { VpnRegistration } from '../enrol/api';
  import { keyText } from '../picture';

  interface Props {
    node: VpnNode;
    renew: (name: string) => Promise<HubResult<VpnRegistration>>;
    now: Date;
    /** Why the broker actions are off; `null` when the broker is ready. */
    reason: string | null;
    /** The id of the line that says `reason`. */
    reasonId: string;
  }

  const { node, renew, now, reason, reasonId }: Props = $props();
  let busy = $state(false);
  let failure = $state<HubError | null>(null);
  let renewed = $state<VpnRegistration | null>(null);

  const line = $derived(
    failure !== null
      ? failure.message
      : renewed !== null
        ? `Renewed. ${keyText({ keyState: 'VALID', keyExpiry: renewed.keyExpiry }, now)}.`
        : '',
  );

  async function run(): Promise<void> {
    if (busy || reason !== null) return;
    busy = true;
    failure = null;
    const result = await renew(node.name);
    busy = false;
    if (result.ok) renewed = result.value;
    else failure = result.error;
  }
</script>

<!-- Renews an untagged operator key (BD-17). The line is always laid out, so a result moves nothing (MOT-007). -->
<div class="renew">
  <FormButton tabindex={0} aria-disabled={busy || reason !== null} aria-busy={busy} aria-describedby={reason === null ? undefined : reasonId} onclick={run}>Renew key</FormButton>
  <p class="outcome" role={failure === null ? 'status' : 'alert'}>{line}</p>
  {#if node.keyState === 'EXPIRED'}
    <p class="expired">A computer whose key has expired is off the VPN, so renew it from another computer.</p>
  {/if}
</div>

<style>
  .renew {
    container-type: inline-size;
    display: flex;
    flex-wrap: wrap;
    grid-column: 1 / -1;
    align-items: center;
    gap: var(--space-2) var(--space-4);
  }

  p {
    margin: 0;
    font-size: var(--text-sm);
  }

  .outcome {
    flex: 1 1 10rem;
    min-block-size: calc(var(--text-sm) * var(--leading-normal));
  }

  /* Two lines below the narrow breakpoint, where the result wraps (MOT-007). */
  @container (max-inline-size: 28rem) {
    .outcome {
      min-block-size: calc(var(--text-sm) * var(--leading-normal) * 2);
    }
  }

  .expired {
    flex: 1 1 100%;
    color: var(--text-muted);
  }
</style>
