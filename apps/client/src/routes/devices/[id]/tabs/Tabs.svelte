<script lang="ts">
  import { untrack } from 'svelte';
  import { nextFocus } from '$lib/sessions/strip/tablist';
  import Icon from '$lib/icons/Icon.svelte';
  import { animate, type Animatable } from '$lib/motion/animate';
  import { springMotion, type InstantMotion, type SpringMotion } from '$lib/motion/options';
  import { motionPreference, type MotionPreference } from '$lib/motion/preference';
  import { press } from '$lib/motion/roles/press';
  import { TABS, hiddenEdges, scrolledToShow, tabIds, type TabName } from './tabs';

  interface Props {
    selected: TabName;
    onSelect: (name: TabName) => void;
    preference?: MotionPreference;
  }

  const { selected, onSelect, preference = motionPreference() }: Props = $props();
  let list = $state<HTMLElement>();
  let indicator = $state<Animatable>();
  let placed = false;

  function tabAt(index: number): HTMLElement | undefined {
    return list?.querySelectorAll<HTMLElement>('[role="tab"]')[index];
  }

  /** The indicator spans the selected tab: `translate` and `scale` from its left edge only (MOT-006). */
  function place(motion: SpringMotion | InstantMotion): void {
    const tab = tabAt(TABS.findIndex((entry) => entry.name === selected));
    if (indicator === undefined || tab === undefined) return;
    const base = indicator instanceof HTMLElement && indicator.offsetWidth > 0 ? indicator.offsetWidth : 1;
    animate(indicator, { translate: { x: tab.offsetLeft, y: 0 }, scale: { x: tab.offsetWidth / base, y: 1 } }, motion);
  }

  /** Tab selection (5.7.3): the indicator moves on `spring-snappy`; in place at once on first paint and under reduced motion. */
  $effect(() => {
    void selected;
    untrack(() => {
      place(placed ? springMotion('snappy', preference.reduced) : { type: 'instant' });
      placed = true;
    });
  });

  /** A font swap or a narrower window changes the tab widths: the indicator follows at once. */
  $effect(() => {
    if (list === undefined || typeof ResizeObserver !== 'function') return;
    const observer = new ResizeObserver(() => {
      place({ type: 'instant' });
    });
    observer.observe(list);
    return () => {
      observer.disconnect();
    };
  });

  /** The strip shows an edge fade where tabs lie past its edge (NAV-005: a focused tab is always scrolled into view). */
  function markEdges(): void {
    if (list === undefined) return;
    const edges = hiddenEdges(list.scrollLeft, list.clientWidth, list.scrollWidth);
    list.toggleAttribute('data-more-before', edges.start);
    list.toggleAttribute('data-more-after', edges.end);
  }

  $effect(() => {
    markEdges();
    if (list === undefined || typeof ResizeObserver !== 'function') return;
    const observer = new ResizeObserver(markEdges);
    observer.observe(list);
    return () => {
      observer.disconnect();
    };
  });

  /** Roving focus (APG tabs, manual activation): arrows, Home and End move focus; Enter or Space selects. */
  function keydown(event: KeyboardEvent): void {
    const tabs = [...(list?.querySelectorAll<HTMLElement>('[role="tab"]') ?? [])];
    const current = tabs.findIndex((tab) => tab === document.activeElement);
    const next = nextFocus(event.key, current < 0 ? 0 : current, tabs.length);
    if (next === null) return;
    event.preventDefault();
    tabs[next]?.focus();
  }
</script>

<div class="tabs" role="tablist" aria-label="Device" tabindex="-1" bind:this={list}
  onkeydown={keydown}
  onscroll={markEdges}
  onfocusin={(event) => {
    if (list === undefined || !(event.target instanceof HTMLElement)) return;
    const fade = Number.parseFloat(getComputedStyle(list).scrollPaddingInlineStart) || 0;
    list.scrollLeft = scrolledToShow(
      { start: list.scrollLeft, size: list.clientWidth },
      { start: event.target.offsetLeft, size: event.target.offsetWidth },
      fade,
    );
  }}
>
  {#each TABS as tab (tab.name)}
    <button
      type="button"
      role="tab"
      id={tabIds(tab.name).tab}
      aria-controls={tabIds(tab.name).panel}
      aria-selected={tab.name === selected}
      tabindex={tab.name === selected ? 0 : -1}
      onclick={() => {
        onSelect(tab.name);
      }}
      {@attach press(preference)}><Icon name={tab.icon} size="sm" />{tab.label}</button
    >
  {/each}
  <span class="indicator" bind:this={indicator} aria-hidden="true"></span>
</div>

<style>
  /* The strip scrolls inside itself when the tabs do not fit, never the page. */
  .tabs {
    position: relative;
    display: flex;
    min-inline-size: 0;
    overflow-x: auto;
    overflow-y: hidden;
    gap: var(--space-1);
    margin-block-end: var(--space-4);
    padding-block-end: var(--border-thin);
    /* The rule is drawn inside the strip, so the clipped strip keeps the underline where the border was. */
    box-shadow: inset 0 calc(var(--border-thin) * -1) 0 var(--border);
    scroll-padding-inline: var(--space-8);
    scrollbar-width: none;
  }

  /* An edge fade where tabs lie past the edge. */
  .tabs:global([data-more-after]) {
    mask-image: linear-gradient(to right, var(--text) calc(100% - var(--space-8)), transparent);
  }

  .tabs:global([data-more-before]) {
    mask-image: linear-gradient(to left, var(--text) calc(100% - var(--space-8)), transparent);
  }

  .tabs:global([data-more-before][data-more-after]) {
    mask-image: linear-gradient(
      to right,
      transparent,
      var(--text) var(--space-8),
      var(--text) calc(100% - var(--space-8)),
      transparent
    );
  }

  /* The icon is hidden from assistive technology; the label alone names the tab (UI-003: the word is always there). */
  button {
    flex: none;
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: 0;
    background: none;
    color: var(--text-muted);
    font: inherit;
    transition: color var(--duration-fast) var(--ease-enter);
  }

  button:hover,
  button[aria-selected='true'] {
    color: var(--text);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: calc(var(--border-focus) * -1);
  }

  /* Scaled from its left edge to the selected tab's width. */
  .indicator {
    position: absolute;
    inset-block-end: 0;
    inset-inline-start: 0;
    inline-size: var(--space-4);
    block-size: var(--border-focus);
    background: var(--accent);
    transform-origin: 0 50%;
    pointer-events: none;
  }
</style>
