<script lang="ts">
  import type { DeviceDetail } from '$lib/devices/api';
  import { placeNear } from '$lib/devices/actions/placement';
  import '$lib/devices/actions/actions.css';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { popover } from '$lib/motion/roles/transitions';
  import { SetupFlow } from './flow';
  import { isBusy, liveText, SHARING_PROMPT, type ScreenSharingCalls, type SetupDevice, type SetupState } from './setup';

  interface Props {
    device: SetupDevice;
    calls: ScreenSharingCalls;
    /** The device with VNC on; the button is passed so the session tab grows from it (MOT-005). It runs while the button is still shown. */
    onReady: (device: DeviceDetail, button: HTMLElement) => void;
    /**
     * Cards and rows show "+ VNC", the width of the VNC button the layout is sized for. The header shows the full text
     * where it has room (wider than 56rem) and "+ VNC" below that, so its reserved rows hold (DEV-021, MOT-007).
     */
    compact?: boolean;
  }

  const { device, calls, onReady, compact = false }: Props = $props();

  let stage = $state<SetupState>({ kind: 'idle' });
  let button = $state<HTMLButtonElement | null>();
  let panel = $state<HTMLElement | null>();
  let root: HTMLElement | undefined;
  // The calls and the callback are fixed for the life of the button.
  // svelte-ignore state_referenced_locally
  const flow = new SetupFlow(
    calls,
    (next) => {
      stage = next;
    },
    (updated) => {
      if (button !== undefined) onReady(updated, button);
    },
  );
  const busy = $derived(isBusy(stage));

  $effect(() => () => {
    flow.dispose();
  });

  /** The panel is fixed and its text changes its size, so it is placed again whenever either changes, and when the window moves. */
  function place(): void {
    if (panel) placeNear(() => button ?? undefined)(panel);
  }

  $effect(() => {
    void stage;
    place();
  });

  $effect(() => {
    if (!panel || typeof ResizeObserver === 'undefined') return;
    const watch = new ResizeObserver(place);
    watch.observe(panel);
    return () => {
      watch.disconnect();
    };
  });

  /** Stops the flow and keeps the focus on the button, which stays enabled while the flow runs. */
  function stop(): void {
    flow.cancel();
    button?.focus();
  }

  /** Escape counts only inside this button and its panel, and not when something else already used it. */
  function onKeydown(event: KeyboardEvent): void {
    if (event.key !== 'Escape' || event.defaultPrevented || stage.kind === 'idle') return;
    if (root?.contains(document.activeElement) !== true) return;
    event.preventDefault();
    stop();
  }
</script>

<svelte:window onscroll={place} onresize={place} />
<svelte:document onkeydown={onKeydown} />

<span class="setup" bind:this={root}>
  <!-- tabindex="0": WebKit skips plain buttons when Tab is pressed unless they are focusable (NAV-005). aria-disabled keeps the focus while the flow runs. -->
  <button
    type="button"
    class="device-action dashed"
    tabindex="0"
    bind:this={button}
    title="Set up screen sharing"
    aria-label="Set up screen sharing for {device.name}"
    aria-busy={busy}
    aria-disabled={busy}
    onclick={() => {
      if (!busy) void flow.start(device);
    }}
    {@attach press()}
  >
    <Icon name="nf-md-plus" size="sm" />
    {#if compact}VNC{:else}<span class="full">Set up screen sharing</span><span class="short">VNC</span>{/if}
  </button>
  <!-- Always mounted, so a change is announced; failures have their own alert. -->
  <span class="live" role="status">{liveText(stage)}</span>
  {#if stage.kind !== 'idle'}
    <div class="panel" bind:this={panel} transition:popover={{ from: button ?? undefined }} {@attach placeNear(() => button ?? undefined)}>
      {#if stage.kind === 'failed'}
        <p role="alert">{stage.message}</p>
        <button type="button" class="device-action" tabindex="0" onclick={stop} {@attach press()}>Close</button>
      {:else}
        <p aria-hidden="true">{liveText(stage)}</p>
        {#if stage.kind === 'waiting' && stage.problem !== null}
          <p role="alert">{stage.problem}</p>
          <button type="button" class="device-action" tabindex="0" onclick={() => void flow.reopenSharing()} {@attach press()}>
            Open System Settings again
          </button>
        {/if}
        {#if stage.kind === 'waiting'}
          <button type="button" class="device-action" tabindex="0" onclick={stop} {@attach press()}>Cancel</button>
        {/if}
      {/if}
    </div>
  {/if}
</span>

<style>
  .setup {
    display: inline-flex;
  }

  /* Not set up: a dashed border and a "+" (DEV-022). */
  .dashed {
    border-style: dashed;
    background: transparent;
  }

  .dashed[aria-disabled='true'] {
    opacity: var(--opacity-disabled);
  }

  /* The full text only where the header is wider than 56rem; the header's own container decides (DEV-021). */
  .short {
    display: inline;
  }

  .full {
    display: none;
  }

  @container (inline-size > 56rem) {
    .full {
      display: inline;
    }

    .short {
      display: none;
    }
  }

  .live {
    position: absolute;
    inline-size: 1px;
    block-size: 1px;
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  /* Fixed, so a card's clipping does not cut it and nothing moves (MOT-007); `placeNear` sets where. */
  .panel {
    position: fixed;
    z-index: var(--layer-popover);
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: var(--space-2);
    inline-size: max-content;
    max-inline-size: min(calc(var(--space-16) * 5), calc(100vw - var(--space-4) * 2));
    margin: 0;
    margin-block-end: var(--space-1);
    padding: var(--space-2);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--material-overlay-bg);
    backdrop-filter: blur(var(--material-overlay-blur)) saturate(var(--material-overlay-saturate));
    box-shadow: var(--elevation-2);
    font-size: var(--text-sm);
  }

  p {
    margin: 0;
    color: var(--text);
  }
</style>
