<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import { motionPreference, type MotionPreference } from '$lib/motion/preference';
  import { press } from '$lib/motion/roles/press';
  import { stripStep, type StripEdges } from './strip';
  import './strip-fade.css';

  interface Props {
    /** The strip the arrows scroll; the arrows sit on its ends, inside a parent with `position: relative`. */
    strip: HTMLElement | undefined;
    /** The ends that have tabs past them, from `stripEdges`: an arrow shows only at such an end. */
    edges: StripEdges;
    preference?: MotionPreference;
  }

  const { strip, edges, preference = motionPreference() }: Props = $props();

  function scroll(direction: 'start' | 'end'): void {
    strip?.scrollBy({ left: stripStep(strip.clientWidth, direction), behavior: preference.reduced ? 'instant' : 'smooth' });
  }
</script>

<!--
  The clear sign that more tabs lie past an end (NAV-005), and the way to them for a pointer that cannot scroll sideways.
  Out of the tab order and hidden from assistive technology: the arrow keys reach each tab, and a focused tab scrolls into view.
-->
{#if edges.start}
  <button type="button" class="arrow start" tabindex="-1" aria-hidden="true" title="Earlier tabs" onclick={() => { scroll('start'); }} {@attach press(preference)}>
    <Icon name="carbon:chevron-left" size="sm" />
  </button>
{/if}
{#if edges.end}
  <button type="button" class="arrow end" tabindex="-1" aria-hidden="true" title="More tabs" onclick={() => { scroll('end'); }} {@attach press(preference)}>
    <Icon name="carbon:chevron-right" size="sm" />
  </button>
{/if}

<style>
  /* 16 px glyph in a 24 px target on the strip's faded end (UI-013). */
  .arrow {
    position: absolute;
    inset-block-start: 50%;
    z-index: var(--layer-sticky);
    display: inline-flex;
    align-items: center;
    justify-content: center;
    inline-size: var(--target-min-desktop);
    block-size: var(--target-min-desktop);
    padding: 0;
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-pill);
    background: var(--surface-overlay);
    color: var(--text);
    box-shadow: var(--elevation-1);
    translate: 0 -50%;
  }

  .start {
    inset-inline-start: 0;
  }

  .end {
    inset-inline-end: 0;
  }

  .arrow:hover {
    background: var(--surface-raised);
  }
</style>
