<script lang="ts">
  import FormButton from '$lib/forms/FormButton.svelte';
  import { useHub } from '$lib/hub/context';
  import Icon from '$lib/icons/Icon.svelte';
  import SettingRow from '$lib/settings/SettingRow.svelte';
  import { vpnApi, type SitePath, type VpnNode } from './api';
  import { enrolApi } from './enrol/api';
  import EnrolModal from './enrol/EnrolModal.svelte';
  import NodeKeys from './keys/NodeKeys.svelte';
  import { VpnLive } from './live.svelte';
  import { brokerReason, readAgo } from './picture';
  import SitePaths from './sites/SitePaths.svelte';

  const hub = useHub();
  const live = new VpnLive(vpnApi(hub));
  const enrol = enrolApi(hub);
  /** The enrolment modal: a site id for its router, `null` for a computer, `undefined` while closed. */
  let enrolling = $state<string | null | undefined>(undefined);
  const REASON_ID = 'vpn-broker-reason';
  let now = $state(new Date());
  let retrying = $state(false);

  // Placeholder rows share the real rows' markup, so the section keeps its height when the picture arrives (MOT-007).
  const WAITING_NODE: VpnNode = {
    name: 'Loading', address: '0.0.0.0', os: '', role: 'OTHER', tags: [], routes: [], waitingRoutes: [], online: false, lastSeen: null,
    keyExpiry: null, keyState: 'NEVER', path: 'UNKNOWN', relayRegion: null, inAnima: true, device: null,
  };
  const WAITING_SITE: SitePath = {
    site: { id: 'waiting', name: 'Loading', subnet: '0.0.0.0/0' },
    router: null, state: 'NO_ROUTER', path: 'UNKNOWN', relayRegion: null, hasPingCheck: true, waitingRouter: null,
  };

  $effect(() => {
    void live.load();
    return () => {
      live.stop();
    };
  });

  $effect(() => {
    const timer = setInterval(() => {
      now = new Date();
    }, 30_000);
    return () => {
      clearInterval(timer);
    };
  });

  /** Try again; the button stays mounted, and aria-disabled, while the hub answers. */
  async function retry(): Promise<void> {
    if (retrying) return;
    retrying = true;
    try {
      await live.load();
    } finally {
      retrying = false;
    }
  }

  const picture = $derived(live.picture);
</script>

{#if picture === undefined && live.error !== null}
  <div class="note">
    <span class="mark"><Icon name="nf-md-alert_circle_outline" size="sm" /></span>
    <p role="alert">Could not load the VPN status.</p>
    <FormButton tabindex={0} aria-disabled={retrying} aria-busy={retrying} onclick={retry}>Retry</FormButton>
  </div>
{:else if picture === undefined}
  <div class="waiting" aria-hidden="true" inert>
    <SitePaths sites={[WAITING_SITE]} broker="READY" onEnrol={() => undefined} reasonId={REASON_ID} />
    <NodeKeys hub={WAITING_NODE} nodes={[]} {now} broker="READY" onEnrol={() => undefined} reasonId={REASON_ID} />
  </div>
{:else if picture === null}
  <SettingRow label="VPN status">
    {#snippet hint()}
      <p class="hint">
        VPN status is not available. The hub has no LocalAPI proxy (ANIMA_TAILSCALE_LOCALAPI), or it has not answered since the hub
        started.
      </p>
    {/snippet}
    Off
  </SettingRow>
{:else}
  <SitePaths sites={picture.sites} broker={picture.broker} onEnrol={(siteId) => { enrolling = siteId; }} reasonId={REASON_ID} />
  <NodeKeys hub={picture.hub} nodes={picture.nodes} {now} broker={picture.broker} onEnrol={() => { enrolling = null; }} reasonId={REASON_ID} />
  {#if brokerReason(picture.broker) !== null}
    <p class="reason" id={REASON_ID}>{brokerReason(picture.broker)}</p>
  {/if}
  {#if enrolling !== undefined}
    <EnrolModal api={enrol} sites={picture.sites} siteId={enrolling} onClose={() => { enrolling = undefined; }} />
  {/if}
  {#if !picture.fresh}
    <p class="stale" role="status">Last read {readAgo(picture.readAt, now).replace(/^Read /, '')}</p>
  {/if}
{/if}

<style>
  .waiting {
    color: transparent;
  }

  .waiting :global(.dot) {
    visibility: hidden;
  }

  .note {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--space-8);
    padding: var(--space-3) var(--space-4);
    border-block-start: var(--border-thin) solid var(--border);
  }

  .mark {
    display: inline-flex;
    color: var(--status-down);
  }

  /* The notice stays --text (MAT-005); the icon beside it marks the error, never colour alone (UI-003). */
  .note p {
    flex: 1 1 0;
    min-inline-size: 0;
    margin: 0;
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  .stale,
  .reason {
    margin: 0;
    padding: var(--space-3) var(--space-4);
    border-block-start: var(--border-thin) solid var(--border);
    color: var(--text-muted);
    font-size: var(--text-sm);
  }
</style>
