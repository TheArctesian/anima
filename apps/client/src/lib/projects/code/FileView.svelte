<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import EmptyState from '$lib/states/EmptyState.svelte';
  import type { CodeFile } from '../forge/types';
  import { sizeWords } from './code';

  interface Props {
    file: CodeFile;
  }

  const { file }: Props = $props();
  const lines = $derived(file.text?.split('\n') ?? []);
</script>

<!-- One file (PRJ-031). Bytes from a repository are untrusted: the text is shown as text, with line numbers, and is never
     rendered or run. A binary or very large file is not shown here; GitHub has it. -->
<div class="file">
  <div class="head">
    <Icon name="carbon:document" size="sm" />
    <span class="name">{file.name}</span>
    <span class="quiet">{file.content === 'TEXT' ? `${String(lines.length)} lines · ` : ''}{sizeWords(file.size)}</span>
    <!-- eslint-disable-next-line svelte/no-navigation-without-resolve -- an address on GitHub -->
    <a href={file.url} target="_blank" rel="noreferrer"><Icon name="simple-icons:github" size="sm" />Open in GitHub</a>
  </div>
  {#if file.content === 'TEXT'}
    <ol class="text" aria-label="Text of {file.name}">
      {#each lines as line, index (index)}<li><code>{line}</code></li>{/each}
    </ol>
  {:else}
    <EmptyState
      title={file.content === 'BINARY' ? 'Not a text file' : 'Too large to show'}
      line={file.content === 'BINARY' ? 'Open it in GitHub.' : 'Over 1 MB. Open it in GitHub.'}
    />
  {/if}
</div>

<style>
  .file {
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-md);
    overflow: hidden;
  }

  .head {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    padding: var(--space-2) var(--space-3);
    border-block-end: var(--border-thin) solid var(--border-subtle);
    background: var(--surface-raised);
    font-size: var(--text-sm);
  }

  .name {
    font-family: var(--font-mono);
  }

  .quiet {
    color: var(--text-tertiary);
    font-size: var(--text-xs);
  }

  a {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
    margin-inline-start: auto;
    color: var(--text-secondary);
    text-decoration: none;
  }

  a:hover {
    color: var(--text);
  }

  .text {
    margin: 0;
    padding: var(--space-2) 0;
    overflow-x: auto;
    background: var(--surface-sunken);
    counter-reset: line;
    list-style: none;
    font-family: var(--font-mono);
    font-size: var(--text-xs);
    line-height: var(--leading-relaxed);
  }

  .text li {
    display: flex;
    counter-increment: line;
    white-space: pre;
  }

  .text li::before {
    content: counter(line);
    flex: none;
    inline-size: var(--space-12);
    padding-inline-end: var(--space-3);
    color: var(--text-quiet);
    text-align: end;
    user-select: none;
  }

  code {
    font: inherit;
  }
</style>
