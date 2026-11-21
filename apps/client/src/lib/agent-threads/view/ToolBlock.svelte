<script lang="ts">
  import { kindName, resultWords, type ToolCall } from '../thread';

  interface Props {
    /** The tool calls that follow each other in a turn. */
    tools: readonly ToolCall[];
    /** Whether each call is open; the key `E` changes it for the whole thread. */
    open: boolean;
  }

  const { tools, open }: Props = $props();

  /** The lines of a patch with the mark of each: removed, added or neither. */
  function patchLines(patch: string): readonly { mark: 'del' | 'add' | 'same'; text: string }[] {
    return patch.split('\n').map((text) => ({
      mark: text.startsWith('-') ? 'del' : text.startsWith('+') ? 'add' : 'same',
      text,
    }));
  }
</script>

<!-- The tool calls of a turn in one block, one line each: the kind, what it does, the result (THR-014). An open call
     shows the patch of an edit, else its output. `details` gives the keyboard and the screen reader the fold. -->
<div class="calls">
  {#each tools as tool, index (index)}
    <details {open}>
      <summary>
        <span class="kind">{kindName(tool.kind)}</span>
        <code>{tool.summary === '' ? tool.name : tool.summary}</code>
        <span class="result" class:bad={tool.failed}>{resultWords(tool)}</span>
      </summary>
      {#if tool.patch !== null}
        <pre>{#each patchLines(tool.patch) as line, at (at)}<span class={line.mark}>{line.text}
</span>{/each}</pre>
      {:else if tool.output !== null}
        <pre>{tool.output}</pre>
      {:else}
        <pre>{tool.input}</pre>
      {/if}
    </details>
  {/each}
</div>

<style>
  .calls {
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-lg);
    overflow: hidden;
  }

  details + details {
    border-block-start: var(--border-thin) solid var(--border-subtle);
  }

  summary {
    display: flex;
    align-items: center;
    gap: var(--space-3);
    min-block-size: var(--control-md);
    padding-inline: var(--space-3);
    font-size: var(--text-sm);
    cursor: pointer;
    list-style: none;
  }

  summary::-webkit-details-marker {
    display: none;
  }

  summary:hover,
  details[open] > summary {
    background: var(--surface-base);
  }

  summary:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: calc(var(--border-focus) * -1);
  }

  .kind {
    flex: none;
    inline-size: var(--space-16);
    color: var(--text-secondary);
  }

  code {
    flex: 1;
    min-inline-size: 0;
    overflow: hidden;
    font-family: var(--font-mono);
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .result {
    color: var(--text-tertiary);
    font-variant-numeric: tabular-nums;
    white-space: nowrap;
  }

  .result.bad {
    color: var(--status-down-text);
  }

  pre {
    max-block-size: calc(var(--control-md) * 12);
    margin: 0;
    padding: var(--space-3) var(--space-4);
    border-block-start: var(--border-thin) solid var(--border-subtle);
    background: var(--surface-sunken);
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    line-height: var(--leading-normal);
    overflow: auto;
  }

  .del {
    color: var(--status-down-text);
  }

  .add {
    color: var(--status-up-text);
  }
</style>
