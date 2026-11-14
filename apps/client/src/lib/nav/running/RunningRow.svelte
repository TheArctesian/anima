<script lang="ts">
  import { resolve } from '$app/paths';
  import type { Pathname } from '$app/types';
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import { ambientMotionAllowed } from '$lib/motion/ambient';
  import { motionPreference } from '$lib/motion/preference';
  import { duration } from '$lib/motion/tokens';
  import { press } from '$lib/motion/roles/press';

  interface Props {
    /** The page that shows the running thing, an app path such as `/devices/abc`. */
    href: Pathname;
    icon: IconName;
    /** What is running, such as a device or project name; cut with an ellipsis. */
    label: string;
    /** A short state, such as "Building · 2 min" or "RDP". */
    detail?: string | undefined;
    /** Still going: a dot that breathes while the window is visible, and holds still with reduced motion (MOT-011). */
    live?: boolean;
    /** The bar shows icons only, so the row's words are its tooltip (UI-015). */
    iconOnly?: boolean;
  }

  const { href, icon, label, detail, live = false, iconOnly = false }: Props = $props();
  const motion = motionPreference();
  let dot: HTMLSpanElement | undefined = $state();
  let visibility = $state(typeof document === 'undefined' ? 'hidden' : document.visibilityState);

  $effect(() => {
    if (!live || dot === undefined || typeof dot.animate !== 'function') return;
    if (!ambientMotionAllowed(visibility, motion.reduced)) return;
    const breathing = dot.animate([{ opacity: 1 }, { opacity: 0.35 }, { opacity: 1 }], { duration: duration.breathe, iterations: Infinity });
    return () => {
      breathing.cancel();
    };
  });
</script>

<svelte:document onvisibilitychange={() => (visibility = document.visibilityState)} />

<!-- One running thing in the bar's foot (UR-7 P2): smaller than a destination, one line, a status mark legible at that size. -->
<a class="row" href={resolve(href)} tabindex="0" title={iconOnly ? [label, detail].filter(Boolean).join(' · ') : undefined} {@attach press()}>
  <span class="mark"><Icon name={icon} size="sm" />{#if live}<span class="dot" bind:this={dot} aria-hidden="true"></span>{/if}</span>
  <span class="words">
    <span class="label">{label}</span>
    {#if detail !== undefined}<span class="detail">{detail}</span>{/if}
  </span>
</a>

<style>
  .row {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--control-sm);
    padding: 0 var(--space-3);
    border-radius: var(--radius-md);
    color: var(--text-secondary);
    font-size: var(--text-xs);
    line-height: var(--leading-tight);
    text-decoration: none;
  }

  .row:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  .row:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: calc(var(--border-focus) * -1);
  }

  .mark {
    position: relative;
    display: inline-flex;
    flex: none;
    color: var(--accent);
  }

  /* The live mark: a filled dot on the icon's corner, ringed in the bar's colour so it reads at this size. */
  .dot {
    position: absolute;
    inset-block-start: calc(var(--space-1) * -1);
    inset-inline-end: calc(var(--space-1) * -1);
    inline-size: var(--space-2);
    block-size: var(--space-2);
    border: var(--border-thin) solid var(--surface-sunken);
    border-radius: var(--radius-pill);
    background: var(--status-up);
  }

  .words {
    display: flex;
    flex: 1;
    gap: var(--space-2);
    min-inline-size: 0;
    white-space: nowrap;
  }

  .label {
    overflow: hidden;
    text-overflow: ellipsis;
    color: var(--text);
  }

  .detail {
    flex: none;
    color: var(--text-tertiary);
  }

  :global(:root[data-nav='icons']) .row {
    justify-content: center;
    padding: 0;
  }

  :global(:root[data-nav='icons']) .words {
    position: absolute;
    inline-size: var(--border-thin);
    block-size: var(--border-thin);
    overflow: hidden;
    clip-path: inset(50%);
  }
</style>
