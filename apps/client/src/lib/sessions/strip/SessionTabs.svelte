<script lang="ts">
  import { flushSync, tick, untrack } from 'svelte';
  import { motionPreference } from '$lib/motion/preference';
  import { flip, measure } from '$lib/motion/roles/flip';
  import { useSessions } from '../sessions.svelte';
  import type { Tab } from '../tabs';
  import { dragTab } from './drag/drag';
  import { grow } from './grow';
  import { enterStrip } from './presence';
  import { nextFocus, panelId } from './tablist';
  import SessionTab from './SessionTab.svelte';

  const sessions = useSessions();
  const preference = motionPreference();
  let strip: HTMLElement | undefined = $state();
  /**
   * The strip is laid out while a tab is open and until the last tab's exit has played, so the exit is seen (MOT-005);
   * only then does it go. Its space goes when the last tab starts to leave (`leaving`), so the page never moves (MOT-007, R-075).
   */
  let reserved = $state(false);
  const leaving = $derived(reserved && sessions.tabs.length === 0);

  /** Whether no slot is drawn in the strip, other than `ending`: no exit is left to play, so the strip can go. */
  function noSlotsLeft(ending?: Element): boolean {
    return strip === undefined || [...strip.querySelectorAll(':scope > [data-slot]')].every((slot) => slot === ending);
  }

  /** The tabs in the strip now, without one that is closing. */
  function slots(): HTMLElement[] {
    return strip === undefined ? [] : [...strip.querySelectorAll<HTMLElement>(':scope > [data-slot]:not([data-closing])')];
  }

  function tabButtons(): HTMLElement[] {
    return slots().flatMap((slot) => [...slot.querySelectorAll<HTMLElement>('[role="tab"]')]);
  }

  /** Whether focus is in the strip, in a tab's menu, or lost, so closing a tab must move it. */
  function focusIsOurs(): boolean {
    const focused = document.activeElement;
    return focused === null || focused === document.body || strip?.contains(focused) === true || focused.closest('[role="menu"]') !== null;
  }

  /** After a tab leaves: the selected tab, else the tab now in its place, else the page's nav item. */
  function refocus(index: number): void {
    const tabs = tabButtons();
    const selected = tabs.find((tab) => tab.getAttribute('aria-selected') === 'true');
    const target = selected ?? tabs[Math.min(index, tabs.length - 1)] ?? document.querySelector<HTMLElement>('nav a[aria-current="page"], nav a');
    target?.focus();
  }

  /** Fixed text and the error name only: the message is the transport's (LOG-007). */
  function warn(text: string, reason: unknown): void {
    console.warn(text, reason instanceof Error ? reason.name : 'unknown');
  }

  /** The closing tab keeps its place while it closes; the others move into the gap (5.7.3). */
  function close(tab: Tab, slot: HTMLElement): void {
    const index = slots().indexOf(slot);
    const moveFocus = focusIsOurs();
    const others = slots().filter((other) => other !== slot);
    const places = measure(others);
    const box = slot.getBoundingClientRect();
    const frame = strip?.getBoundingClientRect();
    slot.dataset.closing = '';
    slot.style.position = 'absolute';
    slot.style.left = `${String(box.left - (frame?.left ?? 0))}px`;
    slot.style.top = `${String(box.top - (frame?.top ?? 0))}px`;
    sessions.close(tab.id).catch((reason: unknown) => {
      warn('Ending the session failed:', reason);
    });
    flushSync();
    flip(places);
    if (moveFocus) refocus(index);
  }

  async function popOut(tab: Tab): Promise<boolean> {
    const index = sessions.tabs.findIndex((open) => open.id === tab.id);
    try {
      await sessions.popOut(tab.id);
      const moveFocus = focusIsOurs();
      flushSync();
      if (moveFocus) refocus(index);
      return !sessions.tabs.some((open) => open.id === tab.id);
    } catch (reason) {
      warn('Moving the session to a window failed:', reason);
      return !sessions.tabs.some((open) => open.id === tab.id);
    }
  }

  function drop(from: number, to: number): void {
    sessions.move(from, to);
    flushSync();
  }

  function key(event: KeyboardEvent, tab: Tab): void {
    if (!(event.target instanceof HTMLElement) || event.target.getAttribute('role') !== 'tab') return;
    if (event.key === 'Delete') {
      const slot = event.target.closest<HTMLElement>('[data-slot]');
      if (slot !== null) close(tab, slot);
      return;
    }
    const tabs = tabButtons();
    const next = nextFocus(event.key, tabs.indexOf(event.target), tabs.length);
    if (next === null) return;
    event.preventDefault();
    tabs[next]?.focus();
  }

  const anyActive = $derived(sessions.active !== null);

  async function appear(element: HTMLElement): Promise<void> {
    const fromHidden = !reserved;
    reserved = true;
    await tick();
    // The last tab may have closed while the strip waited for its place: entering now would undo the exit.
    if (sessions.tabs.length === 0) {
      // No slot was ever drawn, so no exit will end and release the strip.
      if (noSlotsLeft()) reserved = false;
      return;
    }
    enterStrip(element, fromHidden, preference);
  }

  $effect(() => {
    const open = sessions.tabs.length > 0;
    const element = strip;
    untrack(() => {
      if (element === undefined) return;
      if (open) void appear(element);
    });
  });
</script>

<div class="strip" role="tablist" aria-label="Sessions" hidden={!reserved && sessions.tabs.length === 0} class:leaving bind:this={strip}>
  {#each sessions.tabs as tab, index (tab.id)}
    {@const selected = sessions.active?.id === tab.id}
    <div
      class="slot"
      role="none"
      data-slot
      onkeydown={(event) => {
        key(event, tab);
      }}
      transition:grow={{ origin: tab.origin }}
      onoutroend={(event) => {
        if (sessions.tabs.length === 0 && noSlotsLeft(event.currentTarget)) reserved = false;
      }}
      {@attach dragTab({
        tabs: slots,
        bounds: () => strip?.getBoundingClientRect() ?? { left: 0, top: 0, right: 0, bottom: 0 },
        canDragOut: () => sessions.canPopOut && tab.sessionId !== null,
        drop,
        dragOut: () => popOut(tab),
      })}
    >
      <SessionTab
        {tab}
        {selected}
        focusable={selected || (!anyActive && index === 0)}
        panel={panelId(tab.id)}
        onselect={() => {
          sessions.select(tab.id);
        }}
        onclose={(event: MouseEvent) => {
          const slot = event.currentTarget instanceof Element ? event.currentTarget.closest<HTMLElement>('[data-slot]') : null;
          if (slot !== null) close(tab, slot);
        }}
        onpopout={sessions.canPopOut && tab.protocol !== 'WEB'
          ? () => {
              void popOut(tab);
            }
          : undefined}
      />
    </div>
  {/each}
</div>

<style>
  /* The height is reserved whether or not a tab is in the flow (a closing tab is pinned out of it). */
  .strip {
    position: relative;
    box-sizing: border-box;
    --strip-size: calc(var(--target-min-desktop) + var(--space-2));
    min-block-size: var(--strip-size);
    display: flex;
    gap: var(--space-1);
    padding-block-end: var(--space-2);
    overflow-x: auto;
  }

  /* The last tab's exit plays over the page, which is already at its place: the strip's height is taken back from the flow, and it ignores the pointer. */
  .strip.leaving {
    margin-block-end: calc(-1 * var(--strip-size));
    pointer-events: none;
  }

  .strip[hidden] {
    display: none;
  }

  .slot {
    display: inline-flex;
    flex: none;
    align-items: center;
    /* One width for every tab, never wider than the strip's visible box, so a title cannot move the tab or push the close button out (MOT-007). */
    inline-size: min(calc(var(--space-16) * 4), 100%);
    border-radius: var(--radius-md);
    background: var(--surface);
    touch-action: none;
    user-select: none;
  }

  .slot:global([data-dragging]) {
    z-index: var(--layer-popover);
    box-shadow: var(--elevation-2);
  }
</style>
