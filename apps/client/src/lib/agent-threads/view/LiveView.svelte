<script lang="ts">
  import type { CallResult } from '../live/live.svelte';
  import type { Answer, RunState } from '../live/run';
  import { liveParts, type LiveThread } from '../live/timeline';
  import { folderName, programName } from '../thread';
  import AskCard from './AskCard.svelte';
  import Composer from './Composer.svelte';
  import Markdown from './Markdown.svelte';
  import ToolBlock from './ToolBlock.svelte';

  interface Props {
    thread: LiveThread;
    toolsOpen: boolean;
    onSend: (text: string) => Promise<CallResult>;
    onAnswer: (request: string, answer: Answer) => Promise<CallResult>;
    onInterrupt: () => Promise<CallResult>;
    onStop: () => Promise<CallResult>;
  }

  const { thread, toolsOpen, onSend, onAnswer, onInterrupt, onStop }: Props = $props();
  const row = $derived(thread.row);
  const timeline = $derived(liveParts(thread.entries));
  const program = $derived(programName(row.program));
  const folder = $derived(folderName(row.folder));
  const STATE_WORDS: Readonly<Record<RunState, string>> = {
    starting: 'Starting',
    working: 'Working',
    needs_you: 'Needs you',
    idle: 'Your turn',
    stopped: 'Ended',
    unknown: 'Not known',
  };
  const clock = (at: Date): string => at.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });
</script>

<!-- A thread that the runner of this computer drives (TH14): the timeline as the program says it, each request that
     waits, and the composer. Each text of the program is drawn as text (THR-032). -->
<article aria-label={row.title === '' ? 'Untitled thread' : row.title}>
  <header>
    <h2>{row.title === '' ? 'Untitled thread' : row.title}</h2>
    <p class="facts">
      <span>{program}</span>
      <span class="state" data-state={row.state}>{STATE_WORDS[row.state]}</span>
      <span>This computer</span>
      <span class="mono" title={row.folder}>{folder}</span>
      {#if row.model !== null}<span>{row.model}</span>{/if}
      <span>Started {row.started.toLocaleString(undefined, { dateStyle: 'medium', timeStyle: 'short' })}</span>
      {#if row.ended !== null}<span>{row.ended}</span>{/if}
    </p>
  </header>
  <div class="timeline" data-timeline>
    {#if thread.dropped > 0}<p class="turn">{thread.dropped} earlier entries are not shown.</p>{/if}
    {#each timeline as part, index (index)}
      {#if part.kind === 'tools'}
        <ToolBlock tools={part.tools} open={toolsOpen} />
      {:else if part.entry.kind === 'prompt'}
        <div class="you" data-prompt><Markdown text={part.entry.text} /><span class="at">{part.entry.at === null ? '' : clock(part.entry.at)}</span></div>
      {:else if part.entry.kind === 'reply'}
        <div class="reply"><Markdown text={part.entry.text} /></div>
      {:else if part.entry.kind === 'notice'}
        <p class="notice">{part.entry.text}</p>
      {:else if part.entry.kind === 'turn_end'}
        <p class="turn">{part.entry.stopped ? 'Stopped' : 'Turn ended'}{part.entry.at === null ? '' : ` · ${clock(part.entry.at)}`}</p>
      {:else if part.entry.kind === 'reasoning'}
        <details class="reasoning"><summary>Reasoning</summary><p>{part.entry.text}</p></details>
      {/if}
    {:else}
      <p class="turn">The program has said nothing yet.</p>
    {/each}
    {#if thread.streaming !== ''}<p class="reply streaming" aria-live="polite">{thread.streaming}</p>{/if}
    {#each thread.asks as open (open.request)}
      <AskCard {open} {program} {folder} {onAnswer} />
    {/each}
  </div>
  <Composer {row} {onSend} {onInterrupt} {onStop} />
</article>

<style>
  article {
    display: flex;
    flex-direction: column;
    min-inline-size: 0;
  }

  header {
    padding: var(--space-4) var(--space-6);
    border-block-end: var(--border-thin) solid var(--border-subtle);
  }

  h2 {
    margin: 0;
    font-size: var(--text-xl);
  }

  .facts {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-3);
    margin: var(--space-2) 0 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .state[data-state='needs_you'],
  .state[data-state='idle'] {
    color: var(--status-degraded-text);
  }

  .state[data-state='working'] {
    color: var(--status-up-text);
  }

  .mono {
    font-family: var(--font-mono);
  }

  .timeline {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-5) var(--space-6);
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

  .streaming {
    color: var(--text-secondary);
  }

  .notice,
  .reasoning {
    color: var(--text-tertiary);
    font-size: var(--text-sm);
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
</style>
