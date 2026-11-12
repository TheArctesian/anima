<script lang="ts">
  import { resolve } from '$app/paths';
  import WarningBadge from '$lib/devices/actions/WarningBadge.svelte';
  import type { ScreenSharingCalls } from '$lib/devices/screen-sharing/setup';
  import type { DeviceDetail } from '$lib/devices/api';
  import StatusPill from '$lib/devices/card/StatusPill.svelte';
  import OsIcon from '$lib/devices/os/OsIcon.svelte';
  import { skeletonDelay } from '$lib/motion/loading/delay.svelte';
  import type { LoadingClock } from '$lib/motion/loading/loading';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { crossFade } from '$lib/motion/roles/transitions';
  import { sharedItem } from '$lib/motion/shared/shared';
  import type { SessionProtocol } from '$lib/sessions/tabs';
  import type { WebInterface } from '$lib/web-interfaces/api';
  import type { SettingsSection } from '../tabs/tabs';
  import DetailChips from './DetailChips.svelte';
  import HeaderActions from './HeaderActions.svelte';
  import { headerLine, type HeaderDevice } from './header';

  interface Props {
    /** Null while the device loads: the header keeps its size and shows only the back link (D-13). */
    device: HeaderDevice | null;
    /** A protocol that is ready: the button is passed so the session tab grows from it (WP-14 H-1, RA-030, MOT-005). */
    onConnect: (protocol: SessionProtocol, button: HTMLElement) => void;
    /** The Web button (WEB-003); the button is hidden while no route gives this. */
    onOpenWeb?: (webInterface: WebInterface, button: HTMLElement) => void;
    /** The "Set up screen sharing" button (R-079); it is hidden while no route gives this. `onEnabled` shows the updated device. */
    screenSharing?: { readonly calls: ScreenSharingCalls; readonly onEnabled: (device: DeviceDetail) => void };
    /** Opens the Settings tab, at `section` when given (DEV-022). */
    onSettings: (section: SettingsSection | null) => void;
    /** The device is on its way: after `threshold-loading` a skeleton of the header shows until it arrives (5.7.3, Loading). */
    loading?: boolean;
    clock?: LoadingClock;
  }

  const { device, onConnect, onOpenWeb, screenSharing, onSettings, loading = false, clock }: Props = $props();
  const line = $derived(device === null ? null : headerLine(device));
  // The clock is fixed for the life of the header.
  // svelte-ignore state_referenced_locally
  const skeleton = skeletonDelay(() => loading, clock);
</script>

<!--
  Three rows (DEV-020): the back link; the OS logo, the name, the status and the buttons; the details as chips.
  The name, the status and the buttons are always whole (DEV-021). Each row keeps its block size while the device
  loads, so the tabs never move (MOT-007).
-->
<div class="fit">
<header class="head">
  <a class="back" href={resolve('/devices')} {@attach press()}><Icon name="nf-md-chevron_left" size="sm" />Devices</a>
  <div class="title-row">
    {#if device !== null && line !== null}
      <div class="identity">
        <!-- Decorative: the OS chip below says the system in words (UI-003). -->
        <span class="system"><OsIcon os={device.os} distribution={device.distribution} size="lg" decorative /></span>
        <h1 title={device.name} {@attach sharedItem(`device-name:${device.id}`)}>{device.name}</h1>
        <!-- A live region, so a status change is heard as well as seen (D-11). -->
        <span class="status" aria-live="polite">
          <StatusPill status={device.status} deviceId={device.id} />
          <WarningBadge warnings={device.accessWarnings} />
        </span>
      </div>
      <div class="actions-slot">
        <HeaderActions {device} {line} {onConnect} {onOpenWeb} {screenSharing} {onSettings} />
      </div>
    {/if}
  </div>
  <ul class="meta" aria-label="Details">
    {#if device !== null && line !== null}<DetailChips items={line.items} kind={line.kindIcon} os={line.osIcon} />{/if}
  </ul>
  {#if skeleton.visible}
    <!-- Over the header with its rows, after a ghost of the back link: the real header fades in under it without moving (MOT-007). -->
    <div class="head skeleton" aria-hidden="true" transition:crossFade>
      <span class="back ghost"><Icon name="nf-md-chevron_left" size="sm" />Devices</span>
      <div class="title-row">
        <div class="identity"><span class="name"></span><span class="pill"></span></div>
        <span class="actions-place"></span>
      </div>
      <div class="meta"><span class="meta-place"></span></div>
    </div>
  {/if}
</header>
</div>

<style>
  .fit {
    container-type: inline-size;
    min-inline-size: 0;
  }

  .head {
    --actions-rows: 1;
    --actions-block-size: calc(
      var(--target-min-desktop) * var(--actions-rows) + var(--space-2) * (var(--actions-rows) - 1)
    );
    --title-block-size: var(--target-min-touch);
    --chip-block-size: calc(var(--text-sm) * var(--leading-tight) + var(--space-1) * 2);
    position: relative;
    display: grid;
    grid-template-columns: minmax(0, 1fr);
    align-content: start;
    gap: var(--space-2);
    margin-block-end: var(--space-4);
    min-inline-size: 0;
  }

  .skeleton {
    position: absolute;
    inset: 0;
    margin: 0;
    overflow: hidden;
  }

  .ghost {
    visibility: hidden;
  }

  .back {
    justify-self: start;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-block-size: var(--target-min-desktop);
    color: var(--text-secondary);
    font-size: var(--text-sm);
    text-decoration: none;
    transition: color var(--duration-fast) var(--ease-enter);
  }

  .back:hover {
    color: var(--text);
  }

  .back:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  /* The identity takes what the buttons leave; only the name gives way, with an ellipsis and its full text as the tooltip (DEV-021). */
  .title-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    min-block-size: var(--title-block-size);
    min-inline-size: 0;
  }

  .identity {
    display: flex;
    flex: 0 1 auto;
    align-items: center;
    gap: var(--space-3);
    min-block-size: var(--target-min-touch);
    min-inline-size: 0;
  }

  .system {
    flex: none;
    color: var(--text-secondary);
    font-size: var(--text-2xl);
    line-height: var(--leading-tight);
  }

  h1 {
    flex: 0 1 auto;
    min-inline-size: 0;
    margin: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-size: var(--text-2xl);
    font-weight: var(--weight-bold);
    letter-spacing: var(--tracking-display);
    line-height: var(--leading-tight);
  }

  .status,
  .actions-slot {
    flex: none;
  }

  .status {
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .meta {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2);
    min-block-size: var(--chip-block-size);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  /* The skeleton's placeholders: the same rows, so each lies over the part it stands for (5.7.3 Loading). */
  .name,
  .pill,
  .actions-place,
  .meta-place {
    flex: none;
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  .name {
    inline-size: calc(var(--space-16) * 3);
    block-size: var(--space-8);
  }

  .pill {
    inline-size: var(--space-16);
    block-size: var(--space-6);
    border-radius: var(--radius-pill);
  }

  .actions-place {
    inline-size: calc(var(--space-16) * 4);
    max-inline-size: 100%;
    block-size: var(--target-min-desktop);
  }

  .meta-place {
    inline-size: calc(var(--space-16) * 5);
    max-inline-size: 100%;
    block-size: var(--chip-block-size);
    border-radius: var(--radius-pill);
  }

  /*
   * When the header is too narrow for the name, the pill and every button on one row, the buttons take their own row
   * under the name. The container is the header's own width, so the switch follows what fits and not the window; the
   * rows are reserved loading or loaded (DEV-021, MOT-007).
   */
  @container (max-inline-size: 56rem) {
    .head {
      --title-block-size: calc(var(--target-min-touch) + var(--space-2) + var(--actions-block-size));
    }

    .title-row {
      flex-direction: column;
      align-items: stretch;
      justify-content: start;
      gap: var(--space-2);
    }

    .actions-slot,
    .actions-place {
      block-size: var(--actions-block-size);
    }

    .actions-place {
      align-self: start;
    }
  }

  /* Five buttons no longer fit one row: they wrap onto two, and the row reserves both (DEV-021, MOT-007). */
  @container (max-inline-size: 30rem) {
    .head {
      --actions-rows: 2;
    }
  }
</style>
