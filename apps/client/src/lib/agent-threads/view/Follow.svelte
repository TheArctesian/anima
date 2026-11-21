<script lang="ts">
  import { tick, type Snippet } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';

  interface Props {
    /** The thread shown: a new one opens at its newest end. */
    thread: string;
    /** A value that changes when the content grows, so that a reader at the end stays there. */
    grows: unknown;
    children: Snippet;
  }

  const { thread, grows, children }: Props = $props();
  /** How near the end, in px, still counts as reading the newest part. */
  const FOLLOW_SLACK_PX = 24;
  let scroller: HTMLElement | undefined = $state();
  /** The view keeps the newest part in sight until the reader scrolls up; scrolling back to the end follows again. */
  let following = $state(true);

  function scrolled(): void {
    if (scroller) following = scroller.scrollHeight - scroller.scrollTop - scroller.clientHeight <= FOLLOW_SLACK_PX;
  }

  /** A jump, not a glide, so it is the same with reduced motion. */
  async function toEnd(): Promise<void> {
    await tick();
    if (scroller) scroller.scrollTop = scroller.scrollHeight;
  }

  // A thread opens at its newest end, like a coding agent (THR-014).
  $effect(() => {
    void thread;
    following = true;
  });

  // A pane or a content that changes its size (a window, a tool call or reasoning opened, a taller composer) keeps a
  // reader at the end.
  $effect(() => {
    const node = scroller;
    if (node === undefined || typeof ResizeObserver !== 'function') return;
    const observer = new ResizeObserver(() => {
      if (following) node.scrollTop = node.scrollHeight;
    });
    observer.observe(node);
    if (node.firstElementChild !== null) observer.observe(node.firstElementChild);
    return () => {
      observer.disconnect();
    };
  });

  $effect(() => {
    void grows;
    if (following) void toEnd();
  });
</script>

<!-- The scroll area of one thread: it opens at the newest end and follows it while the reader is there; when the reader
     scrolls up, a button takes them back (THR-014). -->
<div class="pane">
  <!-- The thread scrolls on its own; it is reachable by keyboard so it can be read (WCAG 2.1.1). -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <div class="scroll" role="region" tabindex="0" aria-label="Thread content" data-own-keys bind:this={scroller} onscroll={scrolled}>
    {@render children()}
  </div>
  {#if !following}
    <div class="newest">
      <FormButton
        size="sm"
        onclick={() => {
          following = true;
          void toEnd().then(() => scroller?.focus({ preventScroll: true }));
        }}>Newest</FormButton
      >
    </div>
  {/if}
</div>

<style>
  .pane {
    position: relative;
    display: flex;
    flex: 1;
    flex-direction: column;
    min-block-size: 0;
  }

  .scroll {
    flex: 1;
    min-block-size: 0;
    overflow-y: auto;
    overscroll-behavior: contain;
  }

  .scroll:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: calc(var(--border-focus) * -1);
  }

  .newest {
    position: absolute;
    inset-block-end: var(--space-3);
    inset-inline: 0;
    display: flex;
    justify-content: center;
    pointer-events: none;
  }

  .newest > :global(*) {
    pointer-events: auto;
  }
</style>
