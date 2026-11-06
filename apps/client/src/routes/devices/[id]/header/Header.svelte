<script lang="ts">
  import { resolve } from '$app/paths';
  import StatusPill from '$lib/devices/card/StatusPill.svelte';
  import { skeletonDelay } from '$lib/motion/loading/delay.svelte';
  import type { LoadingClock } from '$lib/motion/loading/loading';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { crossFade } from '$lib/motion/roles/transitions';
  import { sharedItem } from '$lib/motion/shared/shared';
  import type { SettingsSection } from '../tabs/tabs';
  import { overflowFade } from './fade';
  import { headerLine, type HeaderDevice } from './header';

  interface Props {
    /** Null while the device loads: the line keeps its size and shows only the back link (D-13). */
    device: HeaderDevice | null;
    /** SSH with an accepted host key: the button is passed so the session tab grows from it (WP-14 H-1, MOT-005). */
    onConnect: (button: HTMLElement) => void;
    /** Opens the Settings tab, at `section` when given (DEV-022). */
    onSettings: (section: SettingsSection | null) => void;
    /** The device is on its way: after `threshold-loading` a skeleton of the line shows until it arrives (5.7.3, Loading). */
    loading?: boolean;
    clock?: LoadingClock;
  }

  const { device, onConnect, onSettings, loading = false, clock }: Props = $props();
  const line = $derived(device === null ? null : headerLine(device));
  // The clock is fixed for the life of the header.
  // svelte-ignore state_referenced_locally
  const skeleton = skeletonDelay(() => loading, clock);
</script>

<!-- One line, in the order of DEV-020; the buttons are last. -->
<header class="line">
  <a class="back" href={resolve('/devices')} {@attach press()}>Devices</a>
  {#if device !== null && line !== null}
    <h1 title={device.name} {@attach sharedItem(`device-name:${device.id}`)}>{device.name}</h1>
    <!-- A live region, so a status change is heard as well as seen (D-11). -->
    <span class="status" aria-live="polite"><StatusPill status={device.status} deviceId={device.id} /></span>
    <p class="details" {@attach overflowFade(line.title)}>
      {#each line.items as item, index (index)}
        <span class="item {item.part}" title={item.text}>{item.text}</span>
      {/each}
    </p>
    <div class="actions">
      <button
        type="button"
        class={line.ssh === 'set-up' ? 'dashed' : 'solid'}
        aria-label={line.ssh === 'set-up' ? 'Set up SSH' : line.ssh === 'test' ? 'Test the SSH connection' : `SSH to ${device.name}`}
        onclick={(event) => {
          if (line.ssh === 'connect') onConnect(event.currentTarget);
          else onSettings(line.ssh === 'test' ? 'test' : 'ssh');
        }}
        {@attach press()}
      >
        {#if line.ssh === 'set-up'}<Icon name="nf-md-plus" size="sm" />{/if}SSH
      </button>
      <!-- RDP comes in Phase 2 (Section 6.3.2): the button only opens its Settings section, never a session. -->
      <button type="button" class="dashed" aria-label="Set up RDP" onclick={() => { onSettings('rdp'); }} {@attach press()}>
        <Icon name="nf-md-plus" size="sm" />RDP
      </button>
      <button type="button" onclick={() => { onSettings(null); }} {@attach press()}>
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

<style>
  /*
   * One row at a block size from tokens, so no font change or late data moves the tabs (D-13, WP-11 H-7). The host and tag
   * details take what the name, the pill and the buttons leave and shrink to nothing first; a name that is too long ends in
   * an ellipsis with its full text as the tooltip (DEV-021).
   */
  .line {
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
    inline-size: calc(var(--space-16) * 3);
    block-size: var(--target-min-desktop);
  }

  /*
   * Below the compact breakpoint four rows of fixed size: the back link and the pill; the name; the details; the buttons. The
   * block size is the same while the device loads and when it arrives (MOT-007).
   */
  :global(:root[data-nav='icons']) .line,
  :global(:root[data-nav='icons']) .skeleton {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr) auto;
    grid-template-rows:
      var(--target-min-touch) calc(var(--text-xl) * var(--leading-tight)) calc(var(--text-sm) * var(--leading-tight))
      var(--target-min-desktop);
    gap: var(--space-2) var(--space-3);
  }

  :global(:root[data-nav='icons']) .line {
    block-size: calc(
      var(--target-min-touch) + var(--text-xl) * var(--leading-tight) + var(--text-sm) * var(--leading-tight) +
        var(--target-min-desktop) + var(--space-2) * 3
    );
  }

  :global(:root[data-nav='icons']) .back {
    grid-area: 1 / 1;
  }

  :global(:root[data-nav='icons']) .status {
    grid-area: 1 / 3;
  }

  :global(:root[data-nav='icons']) h1 {
    grid-area: 2 / 1 / 3 / -1;
  }

  :global(:root[data-nav='icons']) .details,
  :global(:root[data-nav='icons']) .actions {
    grid-column: 1 / -1;
  }

  /* The skeleton has the same rows, so each placeholder lies over the row it stands for (MOT-007, 5.7.3 Loading). */
  :global(:root[data-nav='icons']) .skeleton > .ghost {
    grid-area: 1 / 1;
  }

  :global(:root[data-nav='icons']) .skeleton > .pill {
    grid-area: 1 / 3;
    align-self: center;
  }

  :global(:root[data-nav='icons']) .skeleton > .name {
    grid-area: 2 / 1 / 3 / -1;
    align-self: center;
  }

  :global(:root[data-nav='icons']) .skeleton > .details-place {
    grid-area: 3 / 1 / 4 / -1;
    align-self: center;
  }

  :global(:root[data-nav='icons']) .skeleton > .actions-place {
    grid-area: 4 / 1 / 5 / -1;
    justify-self: start;
  }

  /* The pill and the buttons never shrink (DEV-021). */
  .back,
  .status,
  .actions {
    flex: none;
  }

  .back {
    display: flex;
    align-items: center;
    min-block-size: var(--target-min-desktop);
    color: var(--accent);
    text-decoration: none;
  }

  .back:focus-visible,
  button:focus-visible {
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

  button {
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover {
    background: var(--border);
  }

  .solid {
    border-color: var(--accent);
    background: var(--accent);
    color: var(--bg);
  }

  .solid:hover {
    background: var(--accent-deep);
  }

  /* Not set up: a dashed border and a "+" (DEV-022). */
  .dashed {
    border-style: dashed;
    background: transparent;
  }
</style>
