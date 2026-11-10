<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { clockText } from './timeline';

  interface Props {
    playing: boolean;
    /** Seconds into the recording. */
    position: number;
    /** Length in seconds. */
    duration: number;
    onplay: () => void;
    onpause: () => void;
    onseek: (time: number) => void;
  }

  const { playing, position, duration, onplay, onpause, onseek }: Props = $props();
  const elapsed = $derived(clockText(position));
  const total = $derived(clockText(duration));
</script>

<div class="controls">
  <button type="button" aria-label={playing ? 'Pause' : 'Play'} onclick={playing ? onpause : onplay} {@attach press()}>
    <Icon name={playing ? 'nf-md-pause' : 'nf-md-play'} />
  </button>
  <input
    type="range"
    aria-label="Position in the recording"
    aria-valuetext={`${elapsed} of ${total}`}
    min="0"
    max={duration}
    step="0.1"
    value={Math.min(position, duration)}
    oninput={(event) => {
      onseek(event.currentTarget.valueAsNumber);
    }}
  />
  <span class="time"><span>{elapsed}</span> / <span>{total}</span></span>
</div>

<style>
  .controls {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    padding: var(--space-2) var(--space-3);
    background: var(--surface-raised);
    color: var(--text);
  }

  button {
    display: flex;
    align-items: center;
    justify-content: center;
    min-block-size: var(--target-min-desktop);
    min-inline-size: var(--target-min-desktop);
    border: var(--border-thin) solid var(--accent);
    border-radius: var(--radius-md);
    background: var(--accent);
    color: var(--bg);
    font: inherit;
    transition: background-color var(--duration-press) var(--ease-enter);
  }

  button:active {
    background: var(--accent-deep);
  }

  button:focus-visible,
  input:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  input {
    flex: 1;
    min-inline-size: 0;
    min-block-size: var(--target-min-desktop);
    accent-color: var(--accent);
  }

  .time {
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }
</style>
