<script lang="ts">
  import { tick, untrack, type Snippet } from 'svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import { animate, type Animatable } from '$lib/motion/animate';
  import { tweenMotion } from '$lib/motion/options';
  import { motionPreference, type MotionPreference } from '$lib/motion/preference';
  import { press } from '$lib/motion/roles/press';
  import { flip, measure } from '$lib/motion/roles/flip';

  interface Props {
    /** The button text; the state is carried by `aria-expanded`, not by the text. */
    label: string;
    /** Bindable so that a form can open the section when one of its inputs is wrong. */
    open?: boolean;
    preference?: MotionPreference;
    children: Snippet;
  }

  let { label, open = $bindable(false), preference = motionPreference(), children }: Props = $props();
  const id = $props.id();

  let root: HTMLElement | undefined = $state();
  let region: HTMLElement | undefined = $state();
  /** The content is in the document while it is open and while it fades out. */
  let present = $state(untrack(() => open));
  let leaving = $state(false);
  let turn = 0;

  $effect(() => {
    if (open && !present) present = true;
    if (!open && present && !leaving) present = false;
  });

  const fade = () => tweenMotion('base', 'enter', preference.reduced);

  /** The elements after this section in the same parent: what moves when the section opens or closes. */
  function below(): Animatable[] {
    const found: Animatable[] = [];
    for (let next = root?.nextElementSibling; next; next = next.nextElementSibling) {
      if (next instanceof HTMLElement) found.push(next);
    }
    return found;
  }

  /**
   * Disclosure (5.7.3): the content fades, and what lies below moves by FLIP on `spring-default`
   * instead of animating a height (MOT-006). On close the content leaves the layout at once (it is
   * held in place and fades out), so the FLIP is measured after it left and nothing snaps at the end.
   * Under reduced motion the layout change is instant and the fade takes `duration-base`.
   */
  async function toggle(): Promise<void> {
    const mine = ++turn;
    const first = measure(below());
    if (open) {
      open = false;
      leaving = true;
      if (region) region.style.top = `${String(region.offsetTop)}px`;
      await tick();
      flip(first, { spring: 'default', preference });
      if (region) await animate(region, { opacity: 0 }, fade()).finished;
      if (mine !== turn) return;
      leaving = false;
      present = false;
      return;
    }
    open = true;
    leaving = false;
    present = true;
    await tick();
    flip(first, { spring: 'default', preference });
    if (region) {
      region.style.top = '';
      animate(region, { opacity: 0 }, { type: 'instant' });
      void animate(region, { opacity: 1 }, fade());
    }
  }
</script>

<div class="disclosure" bind:this={root}>
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
  <div {id} class="region" class:leaving hidden={!present} inert={leaving} bind:this={region}>
    {@render children()}
  </div>
</div>

<style>
  .disclosure {
    position: relative;
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
    transition: background-color var(--duration-press) var(--ease-enter);
  }

  button:hover {
    background: var(--border);
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
