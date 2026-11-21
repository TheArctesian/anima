<script lang="ts">
  import { resolve } from '$app/paths';
  import { KEY_WORDS } from '../keys';
  import { STATE_MARKS, type ThreadState } from '../state';
  import { folderName, parts, programName, type Thread } from '../thread';
  import Markdown from './Markdown.svelte';
  import Follow from './Follow.svelte';
  import ToolBlock from './ToolBlock.svelte';

  interface Props {
    thread: Thread;
    /** Whether the tool calls are open (the key `E`). */
    toolsOpen: boolean;
    /** The state that the session file gives (THR-055). */
    state: ThreadState;
  }

  const { thread, toolsOpen, state }: Props = $props();
  const timeline = $derived(parts(thread.entries));
  const clock = (at: Date | null): string => (at === null ? '' : at.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' }));
  /** Agent usage with the folder of the thread chosen, when the folder has a name (THR-027). */
  const usage = $derived(`${resolve('/agents/usage')}${thread.folder === null ? '' : `?repository=${encodeURIComponent(folderName(thread.folder))}`}`);
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
      <span class="state" data-state={state} title={STATE_MARKS[state].detail}>{STATE_MARKS[state].words}</span>
      <span>This computer</span>
      {#if thread.folder !== null}<span class="mono" title={thread.folder}>{folderName(thread.folder)}</span>{/if}
      {#if thread.branch !== null}<span class="mono">{thread.branch}</span>{/if}
      {#if thread.model !== null}<span>{thread.model}</span>{/if}
      {#if thread.started !== null}<span>Started {thread.started.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })}</span>{/if}
    </p>
  </header>
  <Follow thread={thread.id} grows={thread.entries}>
    <div class="timeline" data-timeline>
      {#each timeline as part, index (index)}
        {#if part.kind === 'tools'}
          <ToolBlock tools={part.tools} open={toolsOpen} />
        {:else if part.entry.kind === 'prompt'}
          <div class="you" data-prompt>
            <Markdown text={part.entry.text} />
            {#if part.entry.images > 0}<span class="at">{part.entry.images === 1 ? '1 image' : `${String(part.entry.images)} images`}</span>{/if}
            <span class="at">{clock(part.entry.at)}</span>
          </div>
        {:else if part.entry.kind === 'reply'}
          <div class="reply"><Markdown text={part.entry.text} /></div>
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
  </Follow>
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

  .state[data-state='working']::before {
    background: var(--accent);
  }

  .state[data-state='asks']::before,
  .state[data-state='may_wait']::before,
  .state[data-state='your_turn']::before {
    background: var(--status-degraded);
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
