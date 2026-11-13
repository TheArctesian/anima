<script lang="ts">
  import type { JobView, LogLineView } from './api';
  import { shownLines } from './log';
  import { stateLabel, targetLabel } from './release';

  interface Props {
    job: JobView;
    lines: readonly LogLineView[];
    failed: boolean;
  }

  const { job, lines, failed }: Props = $props();
  const text = $derived(shownLines(lines));
</script>

<section aria-label="Job log">
  <h3>Log: {targetLabel(job.target)} ({stateLabel(job.state)})</h3>
  {#if failed}
    <p role="alert">Anima could not load this log.</p>
  {:else}
    <!-- Always present, so a screen reader announces each new line politely (BLD-081). -->
    <div role="log" aria-live="polite" aria-label="Build log">
      {#if text.length === 0}
        <p class="quiet">No log lines yet.</p>
      {:else}
        <!-- A scrolling log must be reachable by keyboard to be read (WCAG 2.1.1). -->
        <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
        <pre tabindex="0">{text.join('\n')}</pre>
      {/if}
    </div>
  {/if}
</section>

<style>
  h3 {
    margin: 0 0 var(--space-2);
    font-size: var(--text-lg);
  }

  .quiet {
    color: var(--text-secondary);
  }

  pre {
    box-sizing: border-box;
    max-block-size: calc(var(--space-16) * 6);
    margin: 0;
    padding: var(--space-3);
    overflow: auto;
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
