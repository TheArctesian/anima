<script lang="ts">
  import { statusLabel, type DeviceStatus } from '$lib/devices/card/card';

  interface Props {
    status: DeviceStatus;
  }

  const { status }: Props = $props();
</script>

<!-- A dot in the status colour beside the status word: never the colour alone (UI-003). Not the header's pill, which is the one that moves from the card (D-12). -->
<span class="status {status.toLowerCase()}"><span class="dot" aria-hidden="true"></span>{statusLabel(status)}</span>

<style>
  .status {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .dot {
    flex: none;
    inline-size: var(--space-2);
    block-size: var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--tone);
  }

  .up {
    --tone: var(--status-up);
  }

  .down {
    --tone: var(--status-down);
  }

  .degraded {
    --tone: var(--status-degraded);
  }

  .maintenance {
    --tone: var(--status-maintenance);
  }

  .unknown {
    --tone: var(--status-unknown);
  }
</style>
