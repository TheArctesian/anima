<script lang="ts">
  import { tick, untrack, type Snippet } from 'svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import { animate, type Animatable } from '$lib/motion/animate';
  import { tweenMotion } from '$lib/motion/options';
  import { motionPreference, type MotionPreference } from '$lib/motion/preference';
  import { press } from '$lib/motion/roles/press';
  import { flip, measure, type Places } from '$lib/motion/roles/flip';

  interface Props {
    /** The button text; the state is carried by `aria-expanded`, not by the text. */
    label: string;
    /**
     * Bindable so that a form can open the section when one of its inputs is wrong. Opening it by
     * setting this from outside (an effect, an error) shows the content at once and does not animate:
     * only the operator's click does.
     */
    open?: boolean;
    preference?: MotionPreference;
    /** Wraps the button in a heading of this level, as the disclosure pattern allows. */
    level?: 2 | 3 | 4 | 5 | 6;
    /** The section's own icon, before the label (a form section such as SSH). */
    icon?: IconName;
    children: Snippet;
  }

  let { label, open = $bindable(false), preference = motionPreference(), level, icon, children }: Props = $props();
  const id = $props.id();

  let root: HTMLElement | undefined = $state();
  let region: HTMLElement | undefined = $state();
  /** The content is in the document while it is open and while it fades out. */
  let present = $state(untrack(() => open));
  let leaving = $state(false);

  $effect(() => {
    if (open && !present) present = true;
    if (!open && present && !leaving) present = false;
  });

  const fade = () => tweenMotion('base', 'enter', preference.reduced);

  /** The elements after `node` in its parent. */
  function followers(node: Element): Animatable[] {
    const found: Animatable[] = [];
    for (let next = node.nextElementSibling; next; next = next.nextElementSibling) {
      if (next instanceof HTMLElement) found.push(next);
    }
    return found;
  }

  function scrolls(node: Element): boolean {
    return ['auto', 'scroll'].includes(getComputedStyle(node).overflowY);
  }

  /**
   * What the section's change of height can move: the elements after it and after each of its
   * ancestors, up to the nearest scroll container (a modal's body) or the page's body. A form section
   * moves the form's actions and whatever follows the form, such as the Settings tab's delete block.
   */
  function below(): Animatable[] {
    const found: Animatable[] = [];
    for (let node: HTMLElement | null = root ?? null; node && node !== document.body && !scrolls(node); node = node.parentElement) {
      found.push(...followers(node));
    }
    return found;
  }

  /** The nearest scroll container around the section, such as a modal's body; null for the page. */
  function scroller(): HTMLElement | null {
    for (let node = root?.parentElement ?? null; node && node !== document.body; node = node.parentElement) {
      if (scrolls(node)) return node;
    }
    return null;
  }

  /**
   * The scroll container's own children. When a closing section makes the container shorter than its
   * scroll position allows, the browser moves the position back and all of them with it; measured, they
   * spring from where they were, with the clicked control, instead of stepping in one frame.
   */
  function scrollerChildren(): Animatable[] {
    return [...(scroller()?.children ?? [])].filter((child) => child instanceof HTMLElement);
  }

  /**
   * The measured elements whose place on screen changed, and those inside one of them: an element that
   * stayed put while its container moved needs a `translate` of its own, or the container's would carry
   * it away (a row below a closed section, when the scroll position moved back by the section's height).
   * The others keep any motion in flight and get no `translate`, which would make a container the
   * containing block of its fixed children.
   */
  function moved(first: Places): Places {
    const changed = new Set(
      [...first]
        .filter(([element, before]) => {
          const after = element.getBoundingClientRect();
          return after.top !== before.top || after.left !== before.left;
        })
        .map(([element]) => element),
    );
    const carried = (element: Animatable) => [...changed].some((outer) => outer !== element && outer.contains(element));
    return new Map([...first].filter(([element]) => changed.has(element) || carried(element)));
  }

  /**
   * Disclosure (5.7.3): the content fades, and what lies below moves by FLIP on `spring-default`
   * instead of animating a height (MOT-006). On close the content leaves the layout at once (it is
   * held in place and fades out), so the FLIP is measured after it left and nothing snaps at the end.
   * Under reduced motion the layout change is instant and the fade takes `duration-base`. A scroll
   * position the browser moves back when the content gets shorter is part of the same motion.
   */
  async function toggle(): Promise<void> {
    const first = measure([...below(), ...scrollerChildren()]);
    if (open) {
      open = false;
      leaving = true;
      if (region) region.style.top = `${String(region.offsetTop)}px`;
      await tick();
      flip(moved(first), { spring: 'default', preference });
      if (region) await animate(region, { opacity: 0 }, fade()).finished;
      // If it was reopened meanwhile, the effect above puts `present` back at once. A reopen from
      // outside (a form showing an error) did not run the open path, so undo the fade here.
      leaving = false;
      const held = measure(scrollerChildren());
      present = false;
      await tick();
      flip(moved(held), { spring: 'default', preference });
      if (open && region) {
        region.style.top = '';
        animate(region, { opacity: 1 }, { type: 'instant' });
      }
      return;
    }
    open = true;
    leaving = false;
    present = true;
    await tick();
    flip(moved(first), { spring: 'default', preference });
    if (region) {
      animate(region, { opacity: 0 }, { type: 'instant' });
      void animate(region, { opacity: 1 }, fade());
    }
  }
</script>

{#snippet trigger()}
  <button
    type="button"
    aria-expanded={open}
    aria-controls={id}
    onclick={toggle}
    {@attach press(preference)}
  >
    {#if icon !== undefined}
      <span class="mark"><Icon name={icon} /></span>
    {/if}
    <span class="label">{label}</span>
    <span class="chevron"><Icon name={open ? 'nf-md-chevron_down' : 'nf-md-chevron_right'} size="sm" /></span>
  </button>
{/snippet}

<div class="disclosure" bind:this={root}>
  {#if level === undefined}
    {@render trigger()}
  {:else}
    <svelte:element this={`h${String(level)}`} class="heading">{@render trigger()}</svelte:element>
  {/if}
  <div {id} class="region" class:leaving hidden={!present} inert={leaving} bind:this={region}>
    {@render children()}
  </div>
</div>

<style>
  /*
   * Layout containment: the content that fades out after a close, held in place below the section, does
   * not keep a scroll container's extent. The container gets shorter at the close itself, so a scroll
   * position moved back is undone by the same FLIP; if the fading content set the extent, the FLIP's
   * own translate would shorten it again and the content would step.
   */
  .disclosure {
    position: relative;
    contain: layout;
  }

  .heading {
    margin: 0;
    font: inherit;
  }

  /* A full-width row: the section's icon and label at the start, the chevron at the end. */
  button {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--space-3);
    inline-size: 100%;
    min-block-size: var(--space-10);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface);
    color: var(--text);
    font: inherit;
    font-weight: var(--weight-bold);
    text-align: start;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  .mark {
    display: inline-flex;
    color: var(--accent);
  }

  .label {
    flex: 1;
  }

  .chevron {
    display: inline-flex;
    color: var(--text-muted);
  }

  button:hover {
    background: var(--surface-hover);
  }

  button:active {
    transition-duration: var(--duration-press);
    background: var(--accent-tint);
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .region {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding-block: var(--space-4) var(--space-2);
  }

  .region[hidden] {
    display: none;
  }

  /* Out of the layout while it fades, so what is below moves at once and the FLIP has no end snap. */
  .leaving {
    position: absolute;
    inset-inline: 0;
  }
</style>
