<script lang="ts">
  import type { DeviceDetail } from '$lib/devices/api';
  import { placeNear } from '$lib/devices/actions/placement';
  import '$lib/devices/actions/actions.css';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { pulse } from '$lib/motion/roles/pulse';
  import { popover } from '$lib/motion/roles/transitions';
  import { SetupFlow } from './flow';
  import { isBusy, SHARING_PROMPT, type ScreenSharingCalls, type SetupDevice, type SetupState } from './setup';

  interface Props {
    device: SetupDevice;
    calls: ScreenSharingCalls;
    /** The device with VNC on; the button is passed so the session tab grows from it (MOT-005). It runs while the button is still shown. */
    onReady: (device: DeviceDetail, button: HTMLElement) => void;
    /** The header has room for the full text; a card shows "Screen sharing" (R-079). */
    compact?: boolean;
  }

  const { device, calls, onReady, compact = false }: Props = $props();

  let stage = $state<SetupState>({ kind: 'idle' });
  let button = $state<HTMLButtonElement>();
  let panel = $state<HTMLElement>();
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

  $effect(() => () => {
    flow.dispose();
  });

  /** The panel is fixed, so it follows its button when the window scrolls or resizes. */
  function follow(): void {
    if (panel !== undefined) placeNear(() => button)(panel);
  }

  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && stage.kind !== 'idle') flow.cancel();
  }
</script>

<svelte:window onscroll={follow} onresize={follow} />
<svelte:document onkeydown={onKeydown} />

<span class="setup">
  <!-- tabindex="0": WebKit skips plain buttons when Tab is pressed unless they are focusable (NAV-005). -->
  <button
    type="button"
    class="device-action dashed"
    tabindex="0"
    bind:this={button}
    aria-label="Set up screen sharing for {device.name}"
    aria-busy={isBusy(stage)}
    disabled={isBusy(stage)}
    onclick={() => void flow.start(device)}
    {@attach press()}
  >
    <Icon name="nf-md-plus" size="sm" />{compact ? 'Screen sharing' : 'Set up screen sharing'}
  </button>
  {#if stage.kind !== 'idle'}
    <div class="panel" bind:this={panel} transition:popover={{ from: button }} {@attach placeNear(() => button)}>
      {#if stage.kind === 'failed'}
        <p role="alert">{stage.message}</p>
        <button type="button" class="device-action" tabindex="0" onclick={() => { flow.cancel(); }} {@attach press()}>Close</button>
      {:else}
        <p role="status">
          {#if stage.kind === 'waiting'}
            <span class="dot" aria-hidden="true" {@attach pulse()}></span>{SHARING_PROMPT}. Anima asks again every few seconds.
          {:else if stage.kind === 'testing'}
            Checking port 5900.
          {:else}
            Turning on screen sharing.
          {/if}
        </p>
        {#if stage.kind === 'waiting' && stage.problem !== null}
          <p role="alert">{stage.problem}</p>
          <button type="button" class="device-action" tabindex="0" onclick={() => void flow.reopenSharing()} {@attach press()}>
            Open System Settings again
          </button>
        {/if}
        {#if stage.kind === 'waiting'}
          <button type="button" class="device-action" tabindex="0" onclick={() => { flow.cancel(); }} {@attach press()}>Cancel</button>
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

  .dot {
    display: inline-block;
    inline-size: var(--space-2);
    block-size: var(--space-2);
    margin-inline-end: var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--accent);
  }
</style>
