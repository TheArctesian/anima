<script lang="ts">
  import { resolve } from '$app/paths';
  import WarningBadge from '$lib/devices/actions/WarningBadge.svelte';
  import WebMenu from '$lib/devices/actions/WebMenu.svelte';
  import { webInterfacesOf } from '$lib/devices/actions/actions';
  import '$lib/devices/actions/actions.css';
  import ScreenSharingButton from '$lib/devices/screen-sharing/ScreenSharingButton.svelte';
  import type { ScreenSharingCalls } from '$lib/devices/screen-sharing/setup';
  import type { DeviceDetail } from '$lib/devices/api';
  import StatusPill from '$lib/devices/card/StatusPill.svelte';
  import { skeletonDelay } from '$lib/motion/loading/delay.svelte';
  import type { LoadingClock } from '$lib/motion/loading/loading';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { crossFade } from '$lib/motion/roles/transitions';
  import { sharedItem } from '$lib/motion/shared/shared';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import type { WebInterface } from '$lib/web-interfaces/api';
  import type { SettingsSection } from '../tabs/tabs';
  import { overflowFade } from './fade';
  import { headerLine, protocolButton, type HeaderDevice, type ProtocolMode } from './header';

  interface Props {
    /** Null while the device loads: the line keeps its size and shows only the back link (D-13). */
    device: HeaderDevice | null;
    /** A protocol that is ready: the button is passed so the session tab grows from it (WP-14 H-1, RA-030, MOT-005). */
    onConnect: (protocol: SessionProtocol, button: HTMLElement) => void;
    /** The Web button (WEB-003); the button is hidden while no route gives this. */
    onOpenWeb?: (webInterface: WebInterface, button: HTMLElement) => void;
    /** The "Set up screen sharing" button (R-079); it is hidden while no route gives this. `onEnabled` shows the updated device. */
    screenSharing?: { readonly calls: ScreenSharingCalls; readonly onEnabled: (device: DeviceDetail) => void };
    /** Opens the Settings tab, at `section` when given (DEV-022). */
    onSettings: (section: SettingsSection | null) => void;
    /** The device is on its way: after `threshold-loading` a skeleton of the line shows until it arrives (5.7.3, Loading). */
    loading?: boolean;
    clock?: LoadingClock;
  }

  const { device, onConnect, onOpenWeb, screenSharing, onSettings, loading = false, clock }: Props = $props();
  const line = $derived(device === null ? null : headerLine(device));
  // The clock is fixed for the life of the header.
  // svelte-ignore state_referenced_locally
  const skeleton = skeletonDelay(() => loading, clock);
</script>

{#snippet protocolAction(protocol: SessionProtocol, mode: ProtocolMode, shown: HeaderDevice)}
  {@const action = protocolButton(protocol, mode, shown.name)}
  <!-- Off is dashed with a "+" and opens Settings, never a session (DEV-022). -->
  <button
    type="button"
    class="device-action {mode === 'set-up' ? 'dashed' : 'solid'}"
    tabindex="0"
    aria-label={action.label}
    onclick={(event) => {
      if (action.section === null) onConnect(protocol, event.currentTarget);
      else onSettings(action.section);
    }}
    {@attach press()}
  >
    {#if mode === 'set-up'}<Icon name="nf-md-plus" size="sm" />{/if}{protocol}
  </button>
{/snippet}

<!-- One line, in the order of DEV-020; the buttons are last. -->
<div class="fit">
<header class="line">
  <a class="back" href={resolve('/devices')} {@attach press()}>Devices</a>
  {#if device !== null && line !== null}
    <h1 title={device.name} {@attach sharedItem(`device-name:${device.id}`)}>{device.name}</h1>
    <!-- A live region, so a status change is heard as well as seen (D-11). -->
    <span class="status" aria-live="polite">
      <StatusPill status={device.status} deviceId={device.id} />
      <WarningBadge warnings={device.accessWarnings} />
    </span>
    <p class="details" {@attach overflowFade(line.title)}>
      {#each line.items as item, index (index)}
        <span class="item {item.part}" title={item.text}>{item.text}</span>
      {/each}
    </p>
    <div class="actions">
      {@render protocolAction('SSH', line.ssh, device)}
      {@render protocolAction('RDP', line.rdp, device)}
      {#if line.vnc}
        {@render protocolAction('VNC', 'connect', device)}
      {:else if screenSharing !== undefined && line.screenSharing}
        <ScreenSharingButton
          {device}
          calls={screenSharing.calls}
          onReady={(updated, button) => {
            onConnect('VNC', button);
            screenSharing.onEnabled(updated);
          }}
        />
      {/if}
      {#if onOpenWeb !== undefined && webInterfacesOf(device).length > 0}
        <WebMenu deviceName={device.name} interfaces={webInterfacesOf(device)} onOpen={onOpenWeb} />
      {/if}
      <button type="button" class="device-action" tabindex="0" onclick={() => { onSettings(null); }} {@attach press()}>
        <Icon name="nf-md-pencil" size="sm" />Edit
      </button>
    </div>
  {/if}
  {#if skeleton.visible}
    <!-- Over the line, after a ghost of the back link that has its width: the real line fades in under it without moving (MOT-007). -->
    <div class="skeleton" aria-hidden="true" transition:crossFade>
      <span class="back ghost">Devices</span>
      <span class="name"></span>
      <span class="pill"></span>
      <span class="details-place"></span>
      <span class="actions-place"></span>
    </div>
  {/if}
</header>
</div>

<style>
  /*
   * One row at a block size from tokens, so no font change or late data moves the tabs (R-042). The host and tag
   * details take what the name, the pill and the buttons leave and shrink to nothing first; a name that is too long ends in
   * an ellipsis with its full text as the tooltip (DEV-021).
   */
  .fit {
    container-type: inline-size;
    min-inline-size: 0;
  }

  .line {
    --actions-rows: 1;
    --actions-block-size: calc(
      var(--target-min-desktop) * var(--actions-rows) + var(--space-2) * (var(--actions-rows) - 1)
    );
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    block-size: var(--target-min-touch);
    margin-block-end: var(--space-4);
    min-inline-size: 0;
    position: relative;
  }

  .skeleton {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-inline-size: 0;
    overflow: hidden;
  }

  .ghost {
    visibility: hidden;
  }

  .name,
  .pill,
  .details-place,
  .actions-place {
    flex: none;
    border-radius: var(--radius-md);
    background: var(--surface);
  }

  .name {
    inline-size: calc(var(--space-16) * 3);
    block-size: var(--space-6);
  }

  .pill {
    inline-size: var(--space-16);
    block-size: var(--space-6);
    border-radius: var(--radius-pill);
  }

  .details-place {
    flex: 1 1 auto;
    min-inline-size: 0;
    block-size: var(--space-4);
  }

  .actions-place {
    inline-size: calc(var(--space-16) * 4);
    block-size: var(--target-min-desktop);
  }

  /*
   * When the window leaves too little for the name, the pill, the Warning badge and every button on one line, four rows
   * of fixed size: the back link and the pill; the name; the details; the buttons. The container is the header's own
   * width, so the switch follows what fits and not the window (DEV-021). The block size is the same while the device
   * loads and when it arrives (MOT-007).
   */
  @container (max-inline-size: 56rem) {
    .line,
    .skeleton {
      display: grid;
      grid-template-columns: auto minmax(0, 1fr) auto;
      grid-template-rows:
        var(--target-min-touch) calc(var(--text-xl) * var(--leading-tight)) calc(var(--text-sm) * var(--leading-tight))
        var(--actions-block-size);
      gap: var(--space-2) var(--space-3);
    }

    .line {
      block-size: calc(
        var(--target-min-touch) + var(--text-xl) * var(--leading-tight) + var(--text-sm) * var(--leading-tight) +
          var(--actions-block-size) + var(--space-2) * 3
      );
    }

    .back {
      grid-area: 1 / 1;
    }

    .status {
      grid-area: 1 / 3;
    }

    h1 {
      grid-area: 2 / 1 / 3 / -1;
    }

    .details,
    .actions {
      grid-column: 1 / -1;
    }

    /* The skeleton has the same rows, so each placeholder lies over the row it stands for (MOT-007, 5.7.3 Loading). */
    .skeleton > .ghost {
      grid-area: 1 / 1;
    }

    .skeleton > .pill {
      grid-area: 1 / 3;
      align-self: center;
    }

    .skeleton > .name {
      grid-area: 2 / 1 / 3 / -1;
      align-self: center;
    }

    .skeleton > .details-place {
      grid-area: 3 / 1 / 4 / -1;
      align-self: center;
    }

    .skeleton > .actions-place {
      grid-area: 4 / 1 / 5 / -1;
      justify-self: start;
      max-inline-size: 100%;
    }

    .actions,
    .skeleton > .actions-place {
      align-self: start;
      block-size: var(--actions-block-size);
    }

    .actions {
      align-content: start;
      flex-wrap: wrap;
    }
  }

  /* Five buttons no longer fit one row: the buttons wrap onto two, and the row reserves both, loading or loaded (DEV-021, MOT-007). */
  @container (max-inline-size: 30rem) {
    .line {
      --actions-rows: 2;
    }
  }

  /* The pill and the buttons never shrink (DEV-021). */
  .back,
  .status,
  .actions {
    flex: none;
  }

  .status {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .back {
    display: flex;
    align-items: center;
    min-block-size: var(--target-min-desktop);
    color: var(--accent);
    text-decoration: none;
  }

  .back:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  h1 {
    flex: 0 1 auto;
    min-inline-size: 0;
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--text-xl);
    line-height: var(--leading-tight);
  }

  .details {
    flex: 1 1 0;
    display: flex;
    gap: var(--space-2);
    min-inline-size: 0;
    margin: 0;
    overflow: hidden;
    color: var(--text-muted);
    font-size: var(--text-sm);
    white-space: nowrap;
  }

  /* The fade covers the end edge only while the group is cut (DEV-021). */
  .details:global([data-overflowing]) {
    mask-image: linear-gradient(to right, var(--text) calc(100% - var(--space-8)), transparent);
  }

  .item {
    flex: none;
  }

  .ip {
    font-family: var(--font-mono);
  }

  .tag {
    padding: 0 var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--surface-raised);
  }

  .actions {
    display: flex;
    gap: var(--space-2);
  }

  .solid {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--bg);
  }

  .solid:hover,
  .solid:active {
    background: var(--accent-deep);
  }

  /* Not set up: a dashed border and a "+" (DEV-022). */
  .dashed {
    border-style: dashed;
    background: transparent;
  }
</style>
