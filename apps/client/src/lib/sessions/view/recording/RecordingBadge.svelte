<script lang="ts" module>
  /** Mirrors the hub's `RecordingState` (contracts/schema.graphql). */
  export type RecordingState = 'OFF' | 'ON' | 'STOPPED';
</script>

<script lang="ts">
  let { state }: { state: RecordingState } = $props();

  const words: Record<RecordingState, string> = { OFF: '', ON: 'Recording', STOPPED: 'Recording stopped' };
</script>

<!--
  RA-040, D-14. The status element is always in the toolbar and empty while recording is off, so a screen reader announces
  the change to "Recording" and then to "Recording stopped". It is static (MOT-011), solid (MAT-008), and sits in the
  toolbar row, never over the display. The state is in the words and the dot shape, not in colour alone.
-->
<span class="live" role="status">
  {#if state !== 'OFF'}
    <span class="badge" class:stopped={state === 'STOPPED'}>
      <span class="dot" aria-hidden="true"></span>{words[state]}
    </span>
  {/if}
</span>

<style>
  .badge {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-1) var(--space-3);
    border: var(--border-thin) solid var(--status-down);
    border-radius: var(--radius-pill);
    background: var(--status-down-tint);
    color: var(--text);
    font-family: var(--font-ui);
    font-size: var(--text-xs);
    white-space: nowrap;
  }

  .dot {
    inline-size: var(--space-2);
    block-size: var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--status-down);
  }

  .stopped {
    border-color: var(--status-warning);
    background: var(--status-warning-tint);
  }

  /* A square, not a dot, so "stopped" differs from "recording" without colour. */
  .stopped .dot {
    border-radius: 0;
    background: var(--status-warning);
  }
</style>
