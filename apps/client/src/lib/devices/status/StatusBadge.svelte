<script lang="ts">
  import type { Snippet } from 'svelte';
  import { crossFade } from '$lib/motion/roles/transitions';
  import { sharedItem } from '$lib/motion/shared/shared';
  import { statusLabel } from '../card/card';
  import type { CombinedStatus } from './precedence';

  interface Props {
    /** The combined status (DEV-006); a hub status passes as it is. */
    status: CombinedStatus;
    /** Marks the badge for the card-to-detail motion (WP-12 H-1, WP-13 H-2). */
    deviceId: string;
    /** The reasons the status applied, for a HelpTip: it sits beside the badge, so the badge's own text stays the word alone. */
    reasons?: Snippet;
  }

  const { status, deviceId, reasons }: Props = $props();
  const word = $derived(status === 'WARNING' ? 'Warning' : statusLabel(status));
  /** Up and Unknown are the quiet states: a dot and the word, no fill (UI-003 keeps the word). */
  const quiet = $derived(status === 'UP' || status === 'UNKNOWN');
</script>

<!-- A new status cross-fades over the old one in the same place (5.7.3, Status change); the same in reduced motion. -->
<span class="slot" data-device-id={deviceId} {@attach sharedItem(`device-status:${deviceId}`)}>
  {#key status}
    <span class="pill {status.toLowerCase()}" class:quiet transition:crossFade>
      <span class="dot" aria-hidden="true"></span>{word}
    </span>
  {/key}
</span>
{#if reasons}<span class="reasons">{@render reasons()}</span>{/if}

<style>
  /* A fixed size: the longest label fits, so a new status moves nothing (MOT-007). */
  .slot {
    display: inline-grid;
    inline-size: calc(var(--space-16) + var(--space-12));
    block-size: calc(var(--text-sm) * var(--leading-tight) + var(--space-2));
  }

  .reasons {
    display: inline-flex;
    margin-inline-start: var(--space-1);
  }

  .pill {
    grid-area: 1 / 1;
    justify-self: end;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0 var(--space-2);
    border-radius: var(--radius-sm);
    background: var(--wash);
    color: var(--ink);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
    line-height: var(--leading-tight);
  }

  /* Up and Unknown: no fill; Up keeps its colour in the word, as on the website (UR-7). */
  .quiet {
    padding-inline: 0;
    background: none;
    font-size: var(--text-sm);
    font-weight: var(--weight-regular);
  }

  .quiet.unknown {
    color: var(--text-secondary);
  }

  .dot {
    flex: none;
    inline-size: calc(var(--space-2) - var(--border-thin));
    block-size: calc(var(--space-2) - var(--border-thin));
    border-radius: var(--radius-pill);
    background: var(--tone);
  }

  .up {
    --tone: var(--status-up);
    --wash: var(--status-up-tint);
    --ink: var(--status-up-text);
  }

  .down {
    --tone: var(--status-down);
    --wash: var(--status-down-tint);
    --ink: var(--status-down-text);
  }

  .warning {
    --tone: var(--status-warning);
    --wash: var(--status-warning-tint);
    --ink: var(--status-warning-text);
  }

  .degraded {
    --tone: var(--status-degraded);
    --wash: var(--status-degraded-tint);
    --ink: var(--status-degraded-text);
  }

  .maintenance {
    --tone: var(--status-maintenance);
    --wash: var(--status-maintenance-tint);
    --ink: var(--status-maintenance-text);
  }

  .unknown {
    --tone: var(--status-unknown);
    --wash: var(--status-unknown-tint);
    --ink: var(--status-unknown-text);
  }
</style>
