<script lang="ts">
  import { tick, untrack, type Snippet } from 'svelte';
  import Icon from '$lib/icons/Icon.svelte';
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
    children: Snippet;
  }

  let { label, open = $bindable(false), preference = motionPreference(), level, children }: Props = $props();
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
   * ancestors, up to the nearest scroll container (a modal's surface) or the body. A form section
   * moves the form's actions and whatever follows the form, such as the Settings tab's delete block.
   */
  function below(): Animatable[] {
    const found: Animatable[] = [];
    for (let node: HTMLElement | null = root ?? null; node && node !== document.body && !scrolls(node); node = node.parentElement) {
      found.push(...followers(node));
    }
    return found;
  }

  /**
   * The measured elements whose place on screen changed. The others keep any motion in flight and get
   * no `translate`, which would make a container the containing block of its fixed children.
   */
  function moved(first: Places): Places {
    return new Map(
      [...first].filter(([element, before]) => {
        const after = element.getBoundingClientRect();
        return after.top !== before.top || after.left !== before.left;
      }),
    );
  }

  /**
   * Disclosure (5.7.3): the content fades, and what lies below moves by FLIP on `spring-default`
   * instead of animating a height (MOT-006). On close the content leaves the layout at once (it is
   * held in place and fades out), so the FLIP is measured after it left and nothing snaps at the end.
   * Under reduced motion the layout change is instant and the fade takes `duration-base`.
   */
  async function toggle(): Promise<void> {
    const first = measure(below());
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
      present = false;
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
    <Icon name={open ? 'nf-md-chevron_down' : 'nf-md-chevron_right'} size="sm" />
    <span>{label}</span>
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
  .disclosure {
    position: relative;
  }

  .heading {
    margin: 0;
    font: inherit;
  }

  button {
    box-sizing: border-box;
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-block-size: var(--target-min-desktop);
    padding: 0 var(--space-3);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
    color: var(--text);
    font: inherit;
    transition: background-color var(--duration-fast) var(--ease-enter);
  }

  button:hover {
    background: var(--border);
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
    padding-block-start: var(--space-3);
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
