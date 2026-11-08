<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { popover } from '$lib/motion/roles/transitions';
  import { capabilities } from '$lib/platform/platform';
  import PinnedFingerprint from '$lib/web-interfaces/open/PinnedFingerprint.svelte';
  import { pinnedFingerprintToShow } from '$lib/web-interfaces/open/pinned';
  import type { WebInterface } from '$lib/web-interfaces/api';
  import './actions.css';
  import { anchorOf, menuLeftBehind, type MenuAnchor } from './menu-anchor';
  import { placeNear } from './placement';

  interface Props {
    deviceName: string;
    /** At least one: the button opens a single interface directly and lists several (WEB-003). */
    interfaces: readonly WebInterface[];
    /** The button is passed so the web tab can grow from it (MOT-005). */
    onOpen: (webInterface: WebInterface, button: HTMLElement) => void;
  }

  const { deviceName, interfaces, onOpen }: Props = $props();
  const id = $props.id();
  // A browser cannot pin (WEB-008), so its menu shows each pin for the operator to compare with the browser's certificate view (WEB-011).
  // The menu then opens even for one interface, so the pin is seen before the tab opens.
  const showPin = !capabilities().webViewTabs;
  const single = $derived(interfaces.length === 1 && !(showPin && pinnedFingerprintToShow(interfaces[0]) !== null) ? interfaces[0] : undefined);
  let open = $state(false);
  let root: HTMLElement | undefined;
  let button = $state<HTMLButtonElement>();

  function close(): void {
    open = false;
    button?.focus();
  }

  function choose(webInterface: WebInterface): void {
    open = false;
    if (button !== undefined) onOpen(webInterface, button);
  }

  function activate(): void {
    if (single !== undefined) choose(single);
    else open = !open;
  }

  /** Escape closes the open menu wherever focus is: WebKit does not focus a button that a mouse click opened. */
  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && open) close();
  }

  /** A press outside closes the menu; focus goes back to the button only when it was inside the menu. */
  function onOutside(event: PointerEvent): void {
    if (!open || root === undefined || (event.target instanceof Node && root.contains(event.target))) return;
    const hadFocus = root.contains(document.activeElement);
    open = false;
    if (hadFocus) button?.focus();
  }

  /** The panel is fixed where it opened: it closes when the button has moved from there (a scroll) or the width changed (a resize), not on a stray event. */
  function closeOnMove(opened: MenuAnchor, event: Event): void {
    if (event.target instanceof Node && root?.contains(event.target) === true) return;
    if (button === undefined || !menuLeftBehind(opened, button.getBoundingClientRect(), window.innerWidth, event.type === 'resize' ? 'resize' : 'scroll')) return;
    const hadFocus = root?.contains(document.activeElement) === true;
    open = false;
    if (hadFocus) button.focus();
  }

  $effect(() => {
    if (!open || button === undefined) return;
    const opened = anchorOf(button.getBoundingClientRect(), window.innerWidth);
    const onMove = (event: Event): void => {
      closeOnMove(opened, event);
    };
    window.addEventListener('scroll', onMove, true);
    window.addEventListener('resize', onMove);
    return () => {
      window.removeEventListener('scroll', onMove, true);
      window.removeEventListener('resize', onMove);
    };
  });
</script>

<svelte:document onpointerdown={onOutside} onkeydown={onKeydown} />

<span class="web" bind:this={root}>
  <!-- tabindex="0": WebKit skips plain buttons when Tab is pressed unless they are focusable (NAV-005). -->
  <button
    type="button"
    class="device-action"
    tabindex="0"
    bind:this={button}
    aria-label={single === undefined ? `Web interfaces of ${deviceName}` : `Open ${single.label} of ${deviceName}`}
    aria-haspopup={single === undefined ? 'true' : undefined}
    aria-expanded={single === undefined ? open : undefined}
    aria-controls={single === undefined ? `${id}-panel` : undefined}
    onclick={activate}
    {@attach press()}
  >
    <Icon name="nf-md-web" size="sm" />Web
  </button>
  {#if open}
    <div class="panel" id="{id}-panel" role="group" aria-label="Web interfaces of {deviceName}" transition:popover={{ from: button }} {@attach placeNear(() => button)}>
      {#each interfaces as webInterface (webInterface.id)}
        <div class="row">
          <button
            type="button"
            class="choice"
            tabindex="0"
            aria-describedby={showPin && pinnedFingerprintToShow(webInterface) !== null ? `${id}-pin-${webInterface.id}` : undefined}
            onclick={() => {
              choose(webInterface);
            }}
          >
            <span class="label">{webInterface.label}</span>
            <span class="url">{webInterface.url}</span>
          </button>
          {#if showPin}<PinnedFingerprint {webInterface} id="{id}-pin-{webInterface.id}" />{/if}
        </div>
      {/each}
    </div>
  {/if}
</span>

<style>
  .web {
    display: inline-flex;
  }

  /* Fixed, so the card's and the table's clipping does not cut it; `placeNear` sets where. */
  .panel {
    position: fixed;
    z-index: var(--layer-popover);
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    min-inline-size: calc(var(--space-16) * 3);
    max-inline-size: calc(100vw - var(--space-4) * 2);
    max-block-size: calc(var(--space-16) * 5);
    margin: 0;
    margin-block-end: var(--space-1);
    padding: var(--space-2);
    overflow-y: auto;
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--material-overlay-bg);
    backdrop-filter: blur(var(--material-overlay-blur)) saturate(var(--material-overlay-saturate));
    box-shadow: var(--elevation-2);
  }

  /* The pin sits under its choice but outside the button, so it is not part of the button's name. */
  .row {
    display: flex;
    flex-direction: column;
    padding-block-end: var(--space-1);
  }

  .row > :global(.pinned) {
    padding-inline: var(--space-2);
  }

  .choice {
    display: flex;
    flex-direction: column;
    align-items: start;
    min-block-size: var(--target-min-desktop);
    padding: var(--space-1) var(--space-2);
    border: 0;
    border-radius: var(--radius-sm);
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: start;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  .choice:hover {
    background: var(--surface-raised);
  }

  .choice:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .url {
    color: var(--text-muted);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }
</style>
