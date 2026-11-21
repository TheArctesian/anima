<script lang="ts">
  import { untrack } from 'svelte';
  import { nextFocus } from '$lib/sessions/strip/tablist';
  import Icon from '$lib/icons/Icon.svelte';
  import { animate, type Animatable } from '$lib/motion/animate';
  import { springMotion, type InstantMotion, type SpringMotion } from '$lib/motion/options';
  import { motionPreference, type MotionPreference } from '$lib/motion/preference';
  import { press } from '$lib/motion/roles/press';
  import StripArrows from '$lib/scroll/StripArrows.svelte';
  import { NO_HIDDEN_EDGES, stripEdges, type StripEdges } from '$lib/scroll/strip';
  import { scrolledToShow, tabIds, type TabSpec } from './tabs';

  interface Props {
    items: readonly TabSpec[];
    /** The `id` of the selected item. */
    selected: string;
    onSelect: (id: string) => void;
    /** Names the strip for assistive technology, for example "Device". */
    label: string;
    /** Prefixes the tab and panel ids: the panel of item `x` is `<idPrefix>-panel-x`. */
    idPrefix: string;
    preference?: MotionPreference;
  }

  const { items, selected, onSelect, label, idPrefix, preference = motionPreference() }: Props = $props();
  let list = $state<HTMLElement>();
  let indicator = $state<Animatable>();
  let placed = false;

  function tabAt(index: number): HTMLElement | undefined {
    return list?.querySelectorAll<HTMLElement>('[role="tab"]')[index];
  }

  /** The indicator spans the selected tab: `translate` and `scale` from its left edge only (MOT-006). */
  function place(motion: SpringMotion | InstantMotion): void {
    const tab = tabAt(items.findIndex((entry) => entry.id === selected));
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

  /*
   * A strip too narrow for its tabs first drops the icons, so the words fit (UI-003 keeps the words); a strip still too
   * narrow fades the far end and shows an arrow there (NAV-005: a focused tab is always scrolled into view).
   */
  let edges = $state<StripEdges>(NO_HIDDEN_EDGES);
  let compact = $state(false);
  /** The width the tabs need with their icons, read when they stopped fitting; wide enough again, the icons come back. */
  let fullWidth = 0;

  function onEdges(next: StripEdges, strip: HTMLElement): void {
    edges = next;
    const icons = items.some((tab) => tab.icon !== undefined);
    if (!compact && icons && strip.scrollWidth > strip.clientWidth + 1) {
      fullWidth = strip.scrollWidth;
      compact = true;
    } else if (compact && strip.clientWidth >= fullWidth) {
      compact = false;
    }
  }

  /** The tabs change width when the icons go or come back, and the strip's own size does not: the indicator follows. */
  $effect(() => {
    void compact;
    untrack(() => {
      place({ type: 'instant' });
    });
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

<div class="tab-strip">
<div class="tabs" class:compact role="tablist" aria-label={label} tabindex="-1" bind:this={list}
  {@attach stripEdges(onEdges)}
  onkeydown={keydown}
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
  {#each items as tab (tab.id)}
    <button
      type="button"
      role="tab"
      id={tabIds(idPrefix, tab.id).tab}
      aria-controls={tabIds(idPrefix, tab.id).panel}
      aria-selected={tab.id === selected}
      tabindex={tab.id === selected ? 0 : -1}
      onclick={() => {
        onSelect(tab.id);
      }}
      {@attach press(preference)}
      >{#if tab.icon !== undefined}<Icon name={tab.icon} size="sm" />{/if}<span class="word" data-word={tab.label}>{tab.label}</span
      >{#if tab.count !== undefined}<span class="count">{tab.count}</span>{/if}</button
    >
  {/each}
  <span class="indicator" bind:this={indicator} aria-hidden="true"></span>
</div>
<StripArrows strip={list} {edges} {preference} />
</div>

<style>
  /* The arrows sit on the strip's ends. */
  .tab-strip {
    position: relative;
    min-inline-size: 0;
    margin-block-end: var(--space-4);
  }

  /* The strip scrolls inside itself when the tabs do not fit, never the page; `strip-fade.css` fades a hidden end. */
  .tabs {
    position: relative;
    display: flex;
    min-inline-size: 0;
    overflow-x: auto;
    overflow-y: hidden;
    gap: var(--space-1);
    padding-block-end: var(--border-thin);
    /* The rule is drawn inside the strip, so the clipped strip keeps the underline where the border was. */
    box-shadow: inset 0 calc(var(--border-thin) * -1) 0 var(--border-subtle);
  }

  /* Too narrow for the icons: the words alone (UI-003). */
  .compact :global(svg) {
    display: none;
  }

  /* The word is always there (UI-003); a tab is text, not a button. */
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
    color: var(--text-secondary);
    font: inherit;
    transition: color var(--duration-fast) var(--ease-enter);
  }

  button:hover {
    color: var(--text);
  }

  /* The selected tab is bold; the bold word is laid out invisibly under the plain one, so a tab never changes width. */
  button[aria-selected='true'] {
    color: var(--text);
    font-weight: var(--weight-medium);
  }

  .word {
    display: inline-flex;
    flex-direction: column;
    align-items: center;
  }

  .word::after {
    content: attr(data-word);
    block-size: 0;
    overflow: hidden;
    visibility: hidden;
    font-weight: var(--weight-medium);
  }

  .count {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-variant-numeric: tabular-nums;
    font-weight: var(--weight-regular);
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
