<script lang="ts">
  import { animate } from '$lib/motion/animate';
  import { tweenMotion } from '$lib/motion/options';
  import { motionPreference } from '$lib/motion/preference';
  import type { CombinedStatus } from './precedence';

  interface Props {
    status: CombinedStatus;
  }

  const { status }: Props = $props();
  const preference = motionPreference();
  let edge: HTMLElement | undefined;
  let seen: CombinedStatus | undefined;

  /** A new status lights the edge in the new status colour, then it fades over `duration-highlight` (5.7.3); the same in reduced motion. */
  $effect(() => {
    const now = status;
    const was = seen;
    seen = now;
    if (edge === undefined || was === undefined || was === now) return;
    animate(edge, { opacity: 1 }, { type: 'instant' });
    animate(edge, { opacity: 0 }, tweenMotion('highlight', 'exit', preference.reduced));
  });
</script>

<span class="edge {status.toLowerCase()}" bind:this={edge} aria-hidden="true"></span>

<style>
  .edge {
    position: absolute;
    inset: 0;
    border: var(--border-focus) solid var(--tone);
    border-radius: inherit;
    opacity: 0;
    pointer-events: none;
  }

  .up {
    --tone: var(--status-up);
  }

  .down {
    --tone: var(--status-down);
  }

  .warning {
    --tone: var(--status-warning);
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
