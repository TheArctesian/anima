<script lang="ts">
  import { tick } from 'svelte';
  import { resolve } from '$app/paths';
  import FormButton from '$lib/forms/FormButton.svelte';
  import { KEY_WORDS } from './keys';
  import { folderName, parts, programName, type Thread } from './thread';
  import ToolBlock from './ToolBlock.svelte';

  interface Props {
    thread: Thread;
    /** Whether the tool calls are open (the key `E`). */
    toolsOpen: boolean;
    /** Whether the file changed in the last 2 minutes. */
    changedNow: boolean;
  }

  const { thread, toolsOpen, changedNow }: Props = $props();
  const timeline = $derived(parts(thread.entries));
  const clock = (at: Date | null): string => (at === null ? '' : at.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' }));
  /** Agent usage with the folder of the thread chosen, when the folder has a name (THR-027). */
  const usage = $derived(`${resolve('/agents/usage')}${thread.folder === null ? '' : `?repository=${encodeURIComponent(folderName(thread.folder))}`}`);

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
    void thread.id;
    following = true;
  });

  // A pane or a content that changes its size (a window, a tool call or reasoning opened) keeps a reader at the end.
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
    void timeline.length;
    void thread.entries;
    if (following) void toEnd();
  });
</script>

<!-- One thread as a timeline (THR-014): prompts at the trailing side, replies, reasoning folded, the tool calls of a
     turn in one block, the end of each turn. The foot says that the page reads and starts nothing (THR-015). -->
<article aria-label={thread.title ?? 'Untitled thread'}>
  <header>
    <div class="first">
      <h2>{thread.title ?? 'Untitled thread'}</h2>
      <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- the path comes from resolve(); only the query string is added -->
      <a class="usage" href={usage}>Usage of this folder</a>
    </div>
    <p class="facts">
      <span>{programName(thread.program)}</span>
      <span class="state" class:live={changedNow}>{changedNow ? 'Changed now' : 'Not changed now'}</span>
      <span>This computer</span>
      {#if thread.folder !== null}<span class="mono" title={thread.folder}>{folderName(thread.folder)}</span>{/if}
      {#if thread.branch !== null}<span class="mono">{thread.branch}</span>{/if}
      {#if thread.model !== null}<span>{thread.model}</span>{/if}
      {#if thread.started !== null}<span>Started {thread.started.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })}</span>{/if}
    </p>
  </header>
  <div class="pane">
    <!-- The thread scrolls on its own; it is reachable by keyboard so it can be read (WCAG 2.1.1). -->
    <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
    <div class="scroll" role="region" tabindex="0" aria-label="Thread content" data-own-keys bind:this={scroller} onscroll={scrolled}>
      <div class="timeline" data-timeline>
        {#each timeline as part, index (index)}
          {#if part.kind === 'tools'}
            <ToolBlock tools={part.tools} open={toolsOpen} />
          {:else if part.entry.kind === 'prompt'}
            <div class="you" data-prompt>
              <p>{part.entry.text}</p>
              {#if part.entry.images > 0}<span class="at">{part.entry.images === 1 ? '1 image' : `${String(part.entry.images)} images`}</span>{/if}
              <span class="at">{clock(part.entry.at)}</span>
            </div>
          {:else if part.entry.kind === 'reply'}
            <p class="reply">{part.entry.text}</p>
          {:else if part.entry.kind === 'reasoning'}
            <details class="reasoning"><summary>Reasoning</summary><p>{part.entry.text}</p></details>
          {:else if part.entry.kind === 'turn_end'}
            <p class="turn">{part.entry.stopped ? 'Stopped' : 'Turn ended'}{part.entry.at === null ? '' : ` · ${clock(part.entry.at)}`}</p>
          {:else}
            <p class="turn">The program made the earlier part of the thread shorter</p>
          {/if}
        {:else}
          <p class="turn">This session has no prompt and no reply that can be shown.</p>
        {/each}
        {#if thread.dropped > 0}<p class="turn">{thread.dropped} more entries are not shown.</p>{/if}
      </div>
    </div>
    {#if !following}
      <div class="newest">
        <FormButton size="sm" onclick={() => { following = true; void toEnd().then(() => scroller?.focus({ preventScroll: true })); }}>Newest</FormButton>
      </div>
    {/if}
  </div>
  <p class="composer">
    This thread is read from the session file of {programName(thread.program)} on this computer. To continue it, use {programName(thread.program)}.
    Sending a message comes in a later stage.
  </p>
  <ul class="keys" aria-label="Keys">
    {#each KEY_WORDS as [key, words] (key)}<li><kbd>{key}</kbd>{words}</li>{/each}
  </ul>
</article>

<style>
  article {
    display: flex;
    flex: 1;
    flex-direction: column;
    min-inline-size: 0;
    min-block-size: 0;
  }

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

  header {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    padding: var(--space-4) var(--space-6);
    border-block-end: var(--border-thin) solid var(--border-subtle);
  }

  .first {
    display: flex;
    align-items: center;
    gap: var(--space-3);
  }

  h2 {
    flex: 1;
    min-inline-size: 0;
    margin: 0;
    overflow: hidden;
    font-size: var(--text-xl);
    font-weight: var(--weight-medium);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .usage {
    color: var(--accent);
    font-size: var(--text-sm);
    white-space: nowrap;
  }

  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-3);
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .state::before {
    content: '';
    display: inline-block;
    inline-size: var(--space-2);
    block-size: var(--space-2);
    margin-inline-end: var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--status-unknown);
  }

  .state.live::before {
    background: var(--accent);
  }

  .mono {
    font-family: var(--font-mono);
  }

  .timeline {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    max-inline-size: calc(var(--content-form) * 1.25);
    padding: var(--space-6);
  }

  .timeline p {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .you {
    align-self: flex-end;
    max-inline-size: 78%;
    padding: var(--space-3) var(--space-4);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  .at {
    display: block;
    margin-block-start: var(--space-1);
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    text-align: end;
  }

  .reply {
    line-height: var(--leading-normal);
  }

  .reasoning {
    color: var(--text-tertiary);
    font-size: var(--text-sm);
  }

  .reasoning summary {
    cursor: pointer;
  }

  .reasoning p {
    margin-block-start: var(--space-2);
  }

  .turn {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }

  .turn::before,
  .turn::after {
    content: '';
    flex: 1;
    border-block-start: var(--border-thin) solid var(--border-subtle);
  }

  .composer {
    max-inline-size: calc(var(--content-form) * 1.25);
    margin: 0 var(--space-6) var(--space-4);
    padding: var(--space-3) var(--space-4);
    border: var(--border-thin) dashed var(--border-strong);
    border-radius: var(--radius-lg);
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .keys {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-4);
    margin: 0;
    padding: var(--space-2) var(--space-6);
    border-block-start: var(--border-thin) solid var(--border-subtle);
    color: var(--text-tertiary);
    font-size: var(--text-xs);
    list-style: none;
  }

  .keys li {
    white-space: nowrap;
  }

  kbd {
    margin-inline-end: var(--space-1);
    padding-inline: var(--space-1);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-sm);
    font-family: var(--font-mono);
  }
</style>
