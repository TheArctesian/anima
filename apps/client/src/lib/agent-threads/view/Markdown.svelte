<script lang="ts">
  import { markdownBlocks, type Inline } from './markdown';

  interface Props {
    /** Text of a session. It is not trusted: each part is drawn as text. */
    text: string;
  }

  const { text }: Props = $props();
  const blocks = $derived(markdownBlocks(text));
</script>

{#snippet line(parts: readonly Inline[])}
  {#each parts as part, index (index)}
    {#if part.kind === 'code'}<code>{part.text}</code>{:else if part.kind === 'bold'}<strong>{part.text}</strong>{:else}{part.text}{/if}
  {/each}
{/snippet}

<!-- A reply or a prompt as Markdown (THR-019): each block is an element of this page with the text of the session inside
     it as text. Nothing of the session is ever set as markup. -->
<div class="markdown">
  {#each blocks as block, index (index)}
    {#if block.kind === 'code'}
      <pre data-language={block.language === '' ? undefined : block.language}><code>{block.text}</code></pre>
    {:else if block.kind === 'heading'}
      <p class="heading" role="heading" aria-level={Math.min(block.level + 2, 6)}>{@render line(block.inline)}</p>
    {:else if block.kind === 'list'}
      <svelte:element this={block.ordered ? 'ol' : 'ul'}>
        {#each block.items as item, at (at)}<li>{@render line(item)}</li>{/each}
      </svelte:element>
    {:else}
      <p>{@render line(block.inline)}</p>
    {/if}
  {/each}
</div>

<style>
  .markdown {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    min-inline-size: 0;
    line-height: var(--leading-normal);
  }

  p {
    margin: 0;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .heading {
    font-weight: var(--weight-medium);
  }

  ul,
  ol {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    margin: 0;
    padding-inline-start: var(--space-5);
  }

  li {
    overflow-wrap: anywhere;
  }

  code {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
  }

  p code,
  li code {
    padding-inline: var(--space-1);
    border-radius: var(--radius-sm);
    background: var(--surface-sunken);
  }

  pre {
    margin: 0;
    padding: var(--space-3) var(--space-4);
    border: var(--border-thin) solid var(--border-subtle);
    border-radius: var(--radius-md);
    background: var(--surface-sunken);
    overflow-x: auto;
  }
</style>
