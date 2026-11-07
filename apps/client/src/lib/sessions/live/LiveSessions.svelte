<script lang="ts">
  import { tick } from 'svelte';
  import type { Hub } from '$lib/hub/hub';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { listRow, popover } from '$lib/motion/roles/transitions';
  import { reorderMotion } from '$lib/devices/card/reorder.svelte';
  import { liveLabel, nextTabStop, startedTime } from './live';
  import { watchLiveSessions } from './live.svelte';
  import { STOP_REFUSED } from './stopping.svelte';

  interface Props {
    hub: Hub;
    /** The bar shows icons only, so the control needs its words as a tooltip (UI-015). */
    iconOnly?: boolean;
  }

  const { hub, iconOnly = false }: Props = $props();
  // The hub is one for the window's life (the root layout provides it once).
  // svelte-ignore state_referenced_locally
  const live = watchLiveSessions(hub);
  const id = $props.id();
  let open = $state(false);
  let root: HTMLElement | undefined;
  let button = $state<HTMLButtonElement>();
  let panel = $state<HTMLElement>();

  $effect(live.start);
  const track = reorderMotion(() => live.sessions.map((session) => session.id).join('\n'));

  function toggle(): void {
    open = !open;
    // The subscription may be unserved: opening is what refreshes the list.
    if (open) void live.refresh();
  }

  /**
   * Moves the panel out of the bar to the body, whose blur the bar's own backdrop filter would otherwise hide
   * (MAT-002), and places it at the bar's outer edge, level with the control; it still grows from the control (MOT-005).
   */
  function placeBesideBar(node: HTMLElement): void {
    document.body.append(node);
    const edge = (button?.closest('nav') ?? button)?.getBoundingClientRect().right ?? 0;
    const top = button?.getBoundingClientRect().top ?? 0;
    node.style.insetInlineStart = `${String(edge)}px`;
    const room = document.documentElement.clientHeight - node.getBoundingClientRect().height;
    node.style.insetBlockStart = `${String(Math.max(0, Math.min(top, room)))}px`;
  }

  const inside = (target: EventTarget | null): boolean =>
    target instanceof Node && (root?.contains(target) === true || panel?.contains(target) === true);

  /** Stop a session; when its row leaves, focus moves to the next Stop button, else the previous, else the control. */
  async function stop(id: string): Promise<void> {
    const ids = live.sessions.map((session) => session.id);
    const at = ids.indexOf(id);
    const held = panel?.contains(document.activeElement) === true;
    await live.stop(id);
    if (!held || live.sessions.some((session) => session.id === id)) return;
    await tick();
    const next = ids[at + 1] ?? ids[at - 1];
    const target = panel?.querySelector<HTMLElement>(`[data-stop="${next ?? ''}"]`);
    (target ?? button)?.focus();
  }

  const stops = (): HTMLElement[] => [...(panel?.querySelectorAll<HTMLElement>('[data-stop]') ?? [])];

  /** On open focus goes in: the first Stop, or the panel itself when the list is empty (C-3). */
  $effect(() => {
    if (panel) (stops()[0] ?? panel).focus();
  });

  /** Tab goes from the control into the panel, which sits in the body; inside it Tab cycles through the Stop buttons. */
  function onTab(event: KeyboardEvent): void {
    const found = stops();
    const active = document.activeElement;
    const onControl = active === button;
    if (!open || (!onControl && panel?.contains(active) !== true)) return;
    if (onControl && event.shiftKey) return;
    const at = onControl ? -1 : found.findIndex((stop) => stop === active);
    const next = onControl ? { in: 0 } : nextTabStop(at, found.length, event.shiftKey);
    const target = 'in' in next ? found[next.in] : button;
    event.preventDefault();
    (target ?? (onControl ? panel : button))?.focus();
  }

  function close(): void {
    open = false;
    button?.focus();
  }

  /** Escape closes the open popover wherever focus is: WebKit does not focus a button that a mouse click opened. */
  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && open) close();
    else if (event.key === 'Tab') onTab(event);
  }

  /** A press outside closes the popover; focus goes back to the control only when it was inside. */
  function onOutside(event: PointerEvent): void {
    if (!open || inside(event.target)) return;
    const hadFocus = inside(document.activeElement);
    open = false;
    if (hadFocus) button?.focus();
  }
</script>

<svelte:document onpointerdown={onOutside} onkeydown={onKeydown} />

<div class="live" bind:this={root}>
  <!-- tabindex="0": WebKit skips plain buttons when Tab is pressed unless they are focusable (NAV-005). -->
  <button
    type="button"
    class="control"
    tabindex="0"
    bind:this={button}
    title={iconOnly ? liveLabel(live.sessions.length) : undefined}
    aria-label={liveLabel(live.sessions.length)}
    aria-expanded={open}
    aria-controls="{id}-panel"
    onclick={toggle}
    {@attach press()}
  >
    <Icon name="nf-md-access_point" />
    <span class="words" aria-hidden="true">Live sessions</span>
    <span class="count" aria-hidden="true">{live.sessions.length}</span>
  </button>
  {#if open}
    <div class="panel" id="{id}-panel" role="group"
      aria-label="Live sessions"
      tabindex="-1"
      bind:this={panel}
      transition:popover={{ from: button }}
      {@attach placeBesideBar}
    >
      {#if live.loadFailure !== null}
        <p class="failure" role="alert"><Icon name="nf-md-alert" />{live.loadFailure}</p>
      {/if}
      {#each live.sessions as session, index (session.id)}
        {@const failure = live.stopFailure(session.id)}
        <div class="row" transition:listRow={{ index }} {@attach track(session.id)}>
          <div class="facts">
            <strong>{session.deviceName}</strong>
            <span>{session.protocol} · {session.operatorName}</span>
            <span>{session.clientAddr} · {startedTime(session)}</span>
            <!-- The sizer holds the failure line's place in every row, so a refused Stop moves nothing (MOT-007). -->
            <span class="slot">
              <span class="failure sizer" aria-hidden="true"><Icon name="nf-md-alert" />{STOP_REFUSED}</span>
              {#if failure !== null}
                <span class="failure" role="alert"><Icon name="nf-md-alert" />{failure}</span>
              {/if}
            </span>
          </div>
          <button
            type="button"
            class="stop"
            tabindex="0"
            aria-label="Stop session on {session.deviceName}"
            data-stop={session.id}
            aria-busy={live.isStopping(session.id)}
            aria-disabled={live.isStopping(session.id)}
            onclick={() => void stop(session.id)}
            {@attach press()}
          >
            {live.isStopping(session.id) ? 'Stopping' : 'Stop'}
          </button>
        </div>
      {:else}
        {#if live.loadFailure === null}
          <p class="note">No live sessions.</p>
        {/if}
      {/each}
      <!-- Last, so that it appearing moves no row (MOT-007). -->
      {#if live.updatesStopped !== null}
        <p class="failure" role="status"><Icon name="nf-md-alert" />{live.updatesStopped}</p>
      {/if}
    </div>
  {/if}
</div>

<style>
  .live {
    position: relative;
    flex: none;
  }

  .control {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    inline-size: 100%;
    block-size: var(--target-min-touch);
    padding: 0 var(--space-3);
    border: 0;
    border-radius: var(--radius-md);
    background: transparent;
    color: var(--text);
    font: inherit;
    line-height: var(--leading-tight);
    text-align: start;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  .control:hover {
    background: var(--surface-raised);
  }

  .control:active {
    transition-duration: var(--duration-press);
    background: var(--border);
  }

  .control:focus-visible,
  .stop:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .count {
    margin-inline-start: auto;
    font-variant-numeric: tabular-nums;
  }

  /* The words stay in the page as the name (UI-015); the width switches at once, never animated (MOT-006). */
  :global(:root[data-nav='icons']) .words {
    position: absolute;
    inline-size: var(--border-thin);
    block-size: var(--border-thin);
    overflow: hidden;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  :global(:root[data-nav='icons']) .control {
    justify-content: center;
    padding: 0;
  }

  :global(:root[data-nav='icons']) .count {
    margin-inline-start: 0;
  }

  .panel {
    position: fixed;
    z-index: var(--layer-popover);
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-inline-size: calc(var(--space-16) * 4);
    max-inline-size: calc(100vw - var(--space-4) * 2);
    max-block-size: calc(var(--space-16) * 6);
    padding: var(--space-2);
    overflow-y: auto;
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    /* Opaque always: the panel can sit over a live remote display (MAT-008). --border turns to text-muted under prefers-contrast: more (MAT-006). */
    background: var(--surface-raised);
    /* The panel is moved to the body, which sets no colour: --text on --surface-raised is 7.49:1 (UI-002). */
    color: var(--text);
    box-shadow: var(--elevation-2);
    font-size: var(--text-sm);
  }

  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    padding: var(--space-1) var(--space-2);
  }

  .facts {
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
    overflow-wrap: anywhere;
  }

  .slot {
    display: grid;
  }

  .slot > * {
    grid-area: 1 / 1;
  }

  .sizer {
    visibility: hidden;
  }

  .panel:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }

  .stop {
    flex: none;
    box-sizing: border-box;
    min-block-size: var(--target-min-desktop);
    /* Fits "Stopping" as well as "Stop", so busy resizes nothing (MOT-007). */
    min-inline-size: calc(var(--space-16) * 1.5);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font: inherit;
  }

  .stop:hover:not([aria-disabled='true']) {
    background: var(--surface-raised);
  }

  .note {
    margin: 0;
    padding: var(--space-2);
  }

  /* --text on --status-down-tint is 7.9:1 (nord6 #ECEFF4 on #4B4655); --status-down on the panel was 2.1:1. The icon keeps the meaning off colour alone. */
  .failure {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    margin: 0;
    padding: var(--space-1) var(--space-2);
    border-radius: var(--radius-md);
    background: var(--status-down-tint);
    color: var(--text);
  }
</style>
