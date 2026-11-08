<script lang="ts">
  import { tick, untrack } from 'svelte';
  import { animate } from '$lib/motion/animate';
  import { tweenMotion } from '$lib/motion/options';
  import { motionPreference, type MotionPreference } from '$lib/motion/preference';
  import { flip, measure } from '$lib/motion/roles/flip';
  import { followers, moved } from './live-notice';

  interface Props {
    /** Why live updates stopped, or null while they run. */
    message: string | null;
    /** What stopped: "updates" for a device screen, "logs" for the log panel. */
    subject?: 'updates' | 'logs';
    /** Where the space to its neighbours is: above it, below it, or padding inside it (the log panel's rows). */
    gap?: 'start' | 'end' | 'padded';
    preference?: MotionPreference;
  }

  const { message, subject = 'updates', gap = 'start', preference = motionPreference() }: Props = $props();

  let root: HTMLElement | undefined = $state();
  let region: HTMLElement | undefined = $state();
  /** The notice is in the document while it shows and while it fades out. */
  let present = $state(untrack(() => message !== null));
  let leaving = $state(false);
  /** The last text, kept while the notice fades out. */
  let text = $state(untrack(() => message ?? ''));
  let known = false;

  const fade = () => tweenMotion('base', 'enter', preference.reduced);

  /**
   * The leaving branch waits on a screen that clears its `liveError`; none does today, so only the tests reach it.
   * Disclosure (5.7.3, NET-004): the notice fades and what lies below moves by FLIP on `spring-default`, so the list never
   * steps by the notice's height in one frame. On leaving, the notice drops out of the layout at once and fades where it was,
   * so the FLIP has no end snap. Under reduced motion the layout change is instant and the fade takes `duration-base`.
   */
  async function change(next: string | null): Promise<void> {
    const showing = present && !leaving;
    if (next !== null) text = next;
    if (root === undefined) {
      present = next !== null;
      return;
    }
    // Nothing to show, already fading out, or already showing (the text is updated above).
    if (next === null ? !showing : showing) return;
    const first = measure(followers(root));
    if (next === null) {
      leaving = true;
      if (region) region.style.top = `${String(region.offsetTop)}px`;
      await tick();
      flip(moved(first), { spring: 'default', preference });
      if (region) await animate(region, { opacity: 0 }, fade()).finished;
      // Shown again meanwhile: the open path below took over and put `leaving` back.
      if (!leaving) return;
      leaving = false;
      present = false;
      return;
    }
    const reopened = leaving;
    leaving = false;
    present = true;
    await tick();
    if (region) {
      region.style.top = '';
      if (!reopened) animate(region, { opacity: 0 }, { type: 'instant' });
    }
    flip(moved(first), { spring: 'default', preference });
    if (region) void animate(region, { opacity: 1 }, fade());
  }

  $effect(() => {
    const next = message;
    untrack(() => {
      if (known) void change(next);
      known = true;
    });
  });
</script>

<div class="slot" bind:this={root}>
  {#if present}
    <p class="notice" class:leaving class:end={gap === 'end'} class:padded={gap === 'padded'} role="status" bind:this={region}>Live {subject} stopped: {text}</p>
  {/if}
</div>

<style>
  .slot {
    position: relative;
  }

  .notice {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    margin: var(--space-4) 0 0;
    color: var(--text);
  }

  .end {
    margin: 0 0 var(--space-4);
  }

  .padded {
    margin: 0;
    padding: var(--space-2) var(--space-3);
  }

  /* Out of the layout while it fades, so what is below moves at once and the FLIP has no end snap. */
  .leaving {
    position: absolute;
    inset-inline: 0;
    margin: 0;
  }
</style>
