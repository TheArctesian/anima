<script lang="ts">
  import type { ContainerLog } from './api';
  import { lineTime } from './rows';

  /** HOST-001: the last lines of one container, oldest first, as text. The lines are the container's own: never drawn as markup. */
  const { log }: { log: ContainerLog } = $props();
</script>

{#if log.cut}
  <p class="cut" role="status">The log passed 4 MiB: the newest lines are missing. Choose fewer lines.</p>
{/if}
{#if log.lines.length === 0}
  <p class="none">The container wrote no line.</p>
{:else}
  <!-- A focusable scroll region, so the keyboard can scroll it (WCAG 2.1.1). -->
  <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
  <ol class="lines" tabindex="0" aria-label="Log lines, oldest first">
    {#each log.lines as line, index (index)}
      <li><time datetime={line.at ?? undefined}>{lineTime(line.at)}</time><span>{line.text}</span></li>
    {/each}
  </ol>
{/if}

<style>
  .cut,
  .none {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }

  .lines {
    box-sizing: border-box;
    max-block-size: 60vh;
    margin: 0;
    padding: var(--space-2) var(--space-3);
    overflow: auto;
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    color: var(--text);
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    list-style: none;
  }

  .lines:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  li {
    display: flex;
    gap: var(--space-3);
  }

  time {
    flex: none;
    color: var(--text-secondary);
    user-select: none;
  }

  span {
    min-inline-size: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }
</style>
