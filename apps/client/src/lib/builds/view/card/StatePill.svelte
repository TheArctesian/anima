<script lang="ts">
  import { crossFade } from '$lib/motion/roles/transitions';
  import type { StateTone } from './progress';

  interface Props {
    /** The state's word: "Building", "Failed", "Partly published". */
    word: string;
    tone: StateTone;
  }

  const { word, tone }: Props = $props();
</script>

<!-- A new state cross-fades over the old one in the same cell (5.7.3, Status change); the same in reduced motion. -->
<span class="slot">
  {#key word}
    <span class="pill {tone}" transition:crossFade><span class="dot" aria-hidden="true"></span>{word}</span>
  {/key}
</span>

<style>
  .slot {
    display: inline-grid;
  }

  .pill {
    grid-area: 1 / 1;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--wash);
    color: var(--ink);
    font-size: var(--text-sm);
    font-weight: var(--weight-bold);
    line-height: var(--leading-tight);
    white-space: nowrap;
  }

  .dot {
    inline-size: var(--space-2);
    block-size: var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--tone);
  }

  .active {
    --tone: var(--accent);
    --wash: var(--accent-tint);
    --ink: var(--text);
  }

  .done {
    --tone: var(--status-up);
    --wash: var(--status-up-tint);
    --ink: var(--status-up-text);
  }

  .failed {
    --tone: var(--status-down);
    --wash: var(--status-down-tint);
    --ink: var(--status-down-text);
  }

  .waiting {
    --tone: var(--status-degraded);
    --wash: var(--status-degraded-tint);
    --ink: var(--status-degraded-text);
  }

  .stopped {
    --tone: var(--status-unknown);
    --wash: var(--status-unknown-tint);
    --ink: var(--status-unknown-text);
  }
</style>
