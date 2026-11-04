<script lang="ts">
  import { flushSync } from 'svelte';
  import { flip, measure } from '$lib/motion/roles/flip';
  import { useSessions } from '../sessions.svelte';
  import type { Tab } from '../tabs';
  import { dragTab } from './drag';
  import { grow } from './grow';
  import { nextFocus, panelId } from './tablist';
  import SessionTab from './SessionTab.svelte';

  const sessions = useSessions();
  let strip: HTMLElement | undefined = $state();

  /** The tabs in the strip now, without one that is closing. */
  function slots(): HTMLElement[] {
    return strip === undefined ? [] : [...strip.querySelectorAll<HTMLElement>(':scope > [data-slot]:not([data-closing])')];
  }

  function focusActive(): void {
    strip?.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]')?.focus();
  }

  /** Fixed text and the error name only: the message is the transport's (LOG-007). */
  function warn(text: string, reason: unknown): void {
    console.warn(text, reason instanceof Error ? reason.name : 'unknown');
  }

  /** The closing tab keeps its place while it closes; the others move into the gap (5.7.3). */
  function close(tab: Tab, slot: HTMLElement): void {
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
    focusActive();
  }

  async function popOut(tab: Tab): Promise<boolean> {
    try {
      await sessions.popOut(tab.id);
      flushSync();
      focusActive();
      return !sessions.tabs.some((open) => open.id === tab.id);
    } catch (reason) {
      warn('Moving the session to a window failed:', reason);
      return false;
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
    const tabs = slots().flatMap((slot) => [...slot.querySelectorAll<HTMLElement>('[role="tab"]')]);
    const next = nextFocus(event.key, tabs.indexOf(event.target), tabs.length);
    if (next === null) return;
    event.preventDefault();
    tabs[next]?.focus();
  }

  const anyActive = $derived(sessions.active !== null);
</script>

<div class="strip" role="tablist" aria-label="Sessions" hidden={sessions.tabs.length === 0} bind:this={strip}>
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
        onpopout={sessions.canPopOut
          ? () => {
              void popOut(tab);
            }
          : undefined}
      />
    </div>
  {/each}
</div>

<style>
  .strip {
    position: relative;
    display: flex;
    gap: var(--space-1);
    padding-block-end: var(--space-2);
    overflow-x: auto;
  }

  .strip[hidden] {
    display: none;
  }

  .slot {
    display: inline-flex;
    flex: none;
    align-items: center;
    max-inline-size: calc(var(--space-16) * 4);
    border-radius: var(--radius-md);
    background: var(--surface);
    touch-action: none;
    user-select: none;
  }

  .slot:global([data-dragging]) {
    z-index: var(--layer-sticky);
    box-shadow: var(--elevation-2);
  }
</style>
