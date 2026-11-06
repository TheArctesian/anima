<script lang="ts">
  import { animate } from '$lib/motion/animate';
  import { springMotion } from '$lib/motion/options';
  import { motionPreference } from '$lib/motion/preference';
  import { BAR_LENGTH, barSummary, cellsOf, shiftedByNewBeat, type Beat } from './heartbeats';

  interface Props {
    /** The device's last heartbeats, oldest first (`latestBeats`, `addBeat`). */
    beats: readonly Beat[];
  }

  const { beats }: Props = $props();
  const preference = motionPreference();
  const cells = $derived(cellsOf(beats));
  const summary = $derived(barSummary(beats));
  let row: HTMLElement | undefined;
  let before: readonly Beat[] | undefined;

  /** The distance between two neighbouring cells, gap included. */
  function pitchOf(track: HTMLElement): number {
    const [first, second] = track.children;
    if (first === undefined || second === undefined) return 0;
    return second.getBoundingClientRect().left - first.getBoundingClientRect().left;
  }

  /** The cells are already in their new places; start one cell to the right and spring back (MOT-002: from where the row is now). */
  $effect(() => {
    const now = beats;
    const was = before;
    before = now;
    if (row === undefined || was === undefined || !shiftedByNewBeat(was, now)) return;
    const shown = Number.parseFloat(getComputedStyle(row).translate.split(' ')[0] ?? '') || 0;
    animate(row, { translate: { x: shown + pitchOf(row) } }, { type: 'instant' });
    animate(row, { translate: { x: 0 } }, springMotion('default', preference.reduced));
  });
</script>

<div class="bar" role="img" aria-label={summary} style:--bar-length={BAR_LENGTH}>
  <div class="row" bind:this={row} aria-hidden="true">
    {#each cells as status, index (index)}
      <span class="cell {status === null ? 'empty' : status.toLowerCase()}"></span>
    {/each}
  </div>
</div>

<style>
  /*
   * A fixed size from tokens: the number of beats moves nothing (MOT-007). Where the card is narrower than the bar, the
   * bar shrinks to the card and the row stays against the end, so the oldest cells are the ones that are cut.
   */
  .bar {
    box-sizing: border-box;
    display: flex;
    justify-content: flex-end;
    min-inline-size: 0;
    overflow: hidden;
    inline-size: calc(var(--bar-length) * var(--space-1) + (var(--bar-length) - 1) * var(--border-thin));
    block-size: var(--space-4);
  }

  .row {
    flex: none;
    display: flex;
    gap: var(--border-thin);
    block-size: 100%;
  }

  .cell {
    flex: none;
    inline-size: var(--space-1);
    block-size: 100%;
    border-radius: var(--radius-sm);
    background: var(--status-unknown);
  }

  .empty {
    background: var(--surface-raised);
  }

  .up {
    background: var(--status-up);
  }

  .down {
    background: var(--status-down);
  }

  .degraded {
    background: var(--status-degraded);
  }

  .maintenance {
    background: var(--status-maintenance);
  }

  .unknown {
    background: var(--status-unknown);
  }
</style>
