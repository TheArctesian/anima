<script lang="ts">
  import { tick } from 'svelte';
  import type { Hub } from '$lib/hub/hub';
  import Icon from '$lib/icons/Icon.svelte';
  import { press } from '$lib/motion/roles/press';
  import { popover } from '$lib/motion/roles/transitions';
  import { liveLabel, startedTime } from './live';
  import { watchLiveSessions } from './live.svelte';

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

  function close(): void {
    open = false;
    button?.focus();
  }

  /** Escape closes the open popover wherever focus is: WebKit does not focus a button that a mouse click opened. */
  function onKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape' && open) close();
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
      bind:this={panel}
      transition:popover={{ from: button }}
      {@attach placeBesideBar}
    >
      {#if live.loadFailure !== null}
        <p class="note" role="alert">{live.loadFailure}</p>
      {/if}
      {#each live.sessions as session (session.id)}
        {@const failure = live.stopFailure(session.id)}
        <div class="row">
          <div class="facts">
            <strong>{session.deviceName}</strong>
            <span>{session.protocol} · {session.operatorName}</span>
            <span>{session.clientAddr} · {startedTime(session)}</span>
            {#if failure !== null}
              <span class="failure" role="alert">{failure}</span>
            {/if}
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
    background: var(--material-overlay-bg);
    backdrop-filter: blur(var(--material-overlay-blur)) saturate(var(--material-overlay-saturate));
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

  .note,
  .failure {
    margin: 0;
    padding: var(--space-2);
  }

  .failure {
    padding: 0;
    color: var(--status-down);
  }
</style>
