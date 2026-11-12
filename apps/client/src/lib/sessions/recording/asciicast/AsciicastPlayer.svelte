<script lang="ts">
  import { onMount } from 'svelte';
  import Controls from './Controls.svelte';
  import { parseCast, type CastFault } from './parse';
  import { browserClock, createCastPlayer, NOTHING_TO_PLAY, type CastPlayer, type PlaybackView } from './player';
  import { openPlaybackTerminal } from '../../screen/playback';

  interface Props {
    /** The plaintext recording: asciicast v2 text, or its bytes. */
    source: string | Uint8Array;
    /** False when the hub stopped before the end: a cut last line is dropped and the whole lines play. */
    complete: boolean;
    /** The recording was refused (a bad header or line). */
    onfault?: (fault: CastFault) => void;
  }

  const { source, complete, onfault }: Props = $props();
  const parsed = $derived(parseCast(source, complete));
  let host: HTMLElement | undefined = $state();
  let player: CastPlayer | undefined = $state();
  let view: PlaybackView = $state(NOTHING_TO_PLAY);

  /**
   * While playing, the position moves with the clock but nothing announces it, so the slider and the clock text are
   * read again this often; 200 ms is smooth enough for a seconds-long clock and costs one cheap read.
   */
  const POLL_MS = 200;

  $effect(() => {
    if (!parsed.ok) onfault?.(parsed.fault);
  });

  $effect(() => {
    if (host === undefined || !parsed.ok) return;
    const opened = openPlaybackTerminal(host);
    const made = createCastPlayer(opened.terminal, parsed.cast, browserClock);
    const refresh = () => {
      view = made.view();
    };
    const unlisten = made.onchange(refresh);
    refresh();
    player = made;
    return () => {
      unlisten();
      made.pause();
      player = undefined;
      opened.dispose();
    };
  });

  onMount(() => {
    const id = setInterval(() => {
      if (player !== undefined && view.playing) view = player.view();
    }, POLL_MS);
    return () => {
      clearInterval(id);
    };
  });
</script>

{#if parsed.ok}
  <div class="playback">
    <div class="display" bind:this={host}></div>
    <Controls
      playing={view.playing}
      position={view.position}
      duration={view.duration}
      onplay={() => player?.play()}
      onpause={() => player?.pause()}
      onseek={(time) => player?.seek(time)}
    />
  </div>
{:else}
  <p class="refused" role="alert">This recording cannot be read.</p>
{/if}

<style>
  .playback {
    display: flex;
    flex-direction: column;
    min-block-size: 0;
    block-size: 100%;
  }

  .display {
    flex: 1;
    min-block-size: 0;
    overflow: auto;
    background: var(--surface-base);
  }

  .refused {
    margin: 0;
    padding: var(--space-4);
    color: var(--text);
  }
</style>
