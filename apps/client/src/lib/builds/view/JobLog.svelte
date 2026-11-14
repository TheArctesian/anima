<script lang="ts">
  import { tick } from 'svelte';
  import FormButton from '$lib/forms/FormButton.svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import type { JobView, LogLineView } from './api';
  import { stateTone } from './card/progress';
  import StatePill from './card/StatePill.svelte';
  import { isFailureLine, shownLines } from './log';
  import { stateLabel, targetLabel } from './release';

  interface Props {
    job: JobView;
    lines: readonly LogLineView[];
    failed: boolean;
  }

  const { job, lines, failed }: Props = $props();
  const text = $derived(shownLines(lines));

  /** How near the end, in px, still counts as reading the newest line. */
  const FOLLOW_SLACK_PX = 24;

  let scroller: HTMLElement | undefined = $state();
  /** The view keeps the newest line in sight until the operator scrolls up to read; scrolling back down follows again. */
  let following = $state(true);

  function atEnd(node: HTMLElement): boolean {
    return node.scrollHeight - node.scrollTop - node.clientHeight <= FOLLOW_SLACK_PX;
  }

  function scrolled(): void {
    if (scroller) following = atEnd(scroller);
  }

  async function toEnd(): Promise<void> {
    await tick();
    if (scroller) scroller.scrollTop = scroller.scrollHeight;
  }

  // A new job starts at its newest line.
  $effect(() => {
    void job.id;
    following = true;
  });

  $effect(() => {
    void text.length;
    if (following) void toEnd();
  });
</script>

<section class="panel" aria-label="Job log">
  <header>
    <h3><span class="platform" aria-hidden="true"><Icon name="carbon:terminal" /></span>Log: {targetLabel(job.target)}</h3>
    <StatePill word={stateLabel(job.state)} tone={stateTone(job.state)} />
    <span class="facts">
      {#if job.builderName}<span>on {job.builderName}</span>{/if}
      <span>{text.length === 1 ? '1 line' : `${String(text.length)} lines`}</span>
    </span>
  </header>
  {#if failed}
    <p class="problem" role="alert">Anima could not load this log.</p>
  {:else}
    <!-- Always present, so a screen reader announces each new line politely (BLD-081). -->
    <div class="body" role="log" aria-live="polite" aria-label="Build log">
      {#if text.length === 0}
        <p class="quiet">No log lines yet.</p>
      {:else}
        <!-- A scrolling log must be reachable by keyboard to be read (WCAG 2.1.1). -->
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <pre tabindex="0" bind:this={scroller} onscroll={scrolled}>{#each text as line, index (index)}<span class="line" class:failure={isFailureLine(line)}>{line}{index < text.length - 1 ? '\n' : ''}</span>{/each}</pre>
      {/if}
    </div>
    {#if !following && text.length > 0}
      <div class="jump">
        <FormButton tabindex={0} size="sm" onclick={() => { following = true; void toEnd(); }}>Jump to the newest line</FormButton>
      </div>
    {/if}
  {/if}
</section>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-inline-size: 0;
    padding: var(--space-3);
    border-radius: var(--radius-md);
    background: var(--surface-raised);
  }

  header {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-2) var(--space-3);
    align-items: center;
  }

  h3 {
    display: inline-flex;
    gap: var(--space-2);
    align-items: center;
    margin: 0;
    font-size: var(--text-lg);
  }

  .platform {
    display: inline-flex;
    color: var(--text-secondary);
  }

  .facts {
    display: inline-flex;
    gap: var(--space-3);
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .quiet {
    margin: 0;
    color: var(--text-secondary);
  }

  .problem {
    margin: 0;
    color: var(--status-down-text);
  }

  pre {
    box-sizing: border-box;
    block-size: var(--log-height, calc(var(--space-16) * 7));
    margin: 0;
    padding: var(--space-3) var(--space-3) var(--space-3) 0;
    overflow: auto;
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    line-height: var(--leading-normal);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    counter-reset: line;
  }

  pre:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  /* Line numbers hang in a gutter; they are drawn, not copied, and a screen reader skips them. */
  .line {
    position: relative;
    display: block;
    padding-inline-start: 6ch;
    counter-increment: line;
  }

  .line::before {
    position: absolute;
    inset-inline-start: 0;
    inline-size: 4.5ch;
    color: var(--text-tertiary);
    text-align: end;
    content: counter(line) / '';
  }

  .failure {
    background: var(--status-down-tint);
    color: var(--status-down-text);
  }

  .jump {
    display: flex;
    justify-content: flex-end;
  }
</style>
