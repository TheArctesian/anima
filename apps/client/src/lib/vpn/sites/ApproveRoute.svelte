<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import ConfirmModal from '$lib/forms/modal/ConfirmModal.svelte';
  import type { HubResult } from '$lib/hub/hub';
  import type { SitePath, VpnNode } from '../api';

  interface Props {
    site: SitePath['site'];
    /** The router that offers the site subnet and waits for approval. */
    router: VpnNode;
    approve: (siteId: string) => Promise<HubResult<unknown>>;
    /** Why the broker actions are off; `null` when the broker is ready. */
    reason: string | null;
    /** The id of the line that says `reason`. */
    reasonId: string;
  }

  const { site, router, approve, reason, reasonId }: Props = $props();
  let confirming = $state(false);
  /** Each opening of the dialog is a new dialog, so one that finished does not stay finished. */
  let opening = $state(0);
  let approved = $state(false);
</script>

<!-- A router waits for its route to be approved (R-055); the hub shows the result through `vpnChanged` (BD-16). -->
<div class="approve">
  <p class="waits">{router.name} offers {site.subnet} and waits for approval.</p>
  <FormButton
    tabindex={0}
    aria-disabled={reason !== null || approved}
    aria-describedby={reason === null ? undefined : reasonId}
    onclick={() => {
      if (reason !== null || approved) return;
      opening += 1;
      confirming = true;
    }}
  >
    Approve route
  </FormButton>
  <!-- One line is reserved, so "This can take one minute." moves nothing (MOT-007). -->
  <p class="after" role="status">{approved ? 'Approved. This can take one minute.' : ''}</p>
</div>

{#key opening}
  <ConfirmModal
    open={confirming}
    title="Approve {site.subnet}?"
    body="Approve {site.subnet} for {router.name}? The hub and the operator's computers can then reach every address in {site.subnet}."
    confirmLabel="Approve route"
    onConfirm={async () => {
      const done = await approve(site.id);
      if (done.ok) {
        approved = true;
        confirming = false;
      }
      return done;
    }}
    onCancel={() => {
      confirming = false;
    }}
  />
{/key}

<style>
  .approve {
    container-type: inline-size;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2) var(--space-4);
  }

  p {
    margin: 0;
    font-size: var(--text-sm);
  }

  .waits {
    flex: 1 1 100%;
    color: var(--text-muted);
  }

  .after {
    flex: 1 1 10rem;
    min-block-size: calc(var(--text-sm) * var(--leading-normal));
    color: var(--text-muted);
  }

  /* Two lines below the narrow breakpoint, where the text wraps (MOT-007). */
  @container (max-inline-size: 28rem) {
    .after {
      min-block-size: calc(var(--text-sm) * var(--leading-normal) * 2);
    }
  }
</style>
