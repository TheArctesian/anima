<script lang="ts">
  import { crossFade } from '$lib/motion/roles/transitions';
  import { statusLabel, type DeviceStatus } from './card';

  interface Props {
    status: DeviceStatus;
    /** Marks the pill for the card-to-detail motion (WP-12 H-1). */
    deviceId: string;
  }

  const { status, deviceId }: Props = $props();
</script>

<!-- A new status cross-fades over the old one in the same place (5.7.3, Status change); the same in reduced motion. -->
<span class="slot" data-device-id={deviceId}>
  {#key status}
    <span class="pill {status.toLowerCase()}" transition:crossFade>
      <span class="dot" aria-hidden="true"></span>{statusLabel(status)}
    </span>
  {/key}
</span>

<style>
  /* A fixed size: the longest label fits, so a new status moves nothing (MOT-007). */
  .slot {
    display: inline-grid;
    inline-size: calc(var(--space-16) * 2);
    block-size: calc(var(--text-sm) * var(--leading-tight) + var(--space-2));
  }

  .pill {
    grid-area: 1 / 1;
    justify-self: start;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: 0 var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--wash);
    color: var(--text);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  .dot {
    inline-size: var(--space-2);
    block-size: var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--tone);
  }

  .up {
    --tone: var(--status-up);
    --wash: var(--status-up-tint);
  }

  .down {
    --tone: var(--status-down);
    --wash: var(--status-down-tint);
  }

  .degraded {
    --tone: var(--status-degraded);
    --wash: var(--status-degraded-tint);
  }

  .maintenance {
    --tone: var(--status-maintenance);
    --wash: var(--status-maintenance-tint);
  }

  .unknown {
    --tone: var(--status-unknown);
    --wash: var(--status-unknown-tint);
  }
</style>
