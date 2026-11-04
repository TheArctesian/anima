<script lang="ts">
  import { normalizeTags } from '../values';

  interface Props {
    tags: readonly string[];
  }

  let { tags = $bindable() }: Props = $props();
  let draft = $state('');

  /** Adds the typed text as a tag (trimmed, no duplicates, T1's rules). */
  function commit(): void {
    if (draft.trim() !== '') tags = normalizeTags([...tags, draft]);
    draft = '';
  }

  function keydown(event: KeyboardEvent): void {
    if (event.key === 'Enter' || event.key === ',') {
      event.preventDefault();
      commit();
    } else if (event.key === 'Backspace' && draft === '') {
      tags = tags.slice(0, -1);
    }
  }
</script>

<div class="field">
  <label for="device-tags">Tags</label>
  <div class="box">
    {#each tags as tag (tag)}
      <span class="chip">
        {tag}
        <button type="button" aria-label="Remove tag {tag}" onclick={() => { tags = tags.filter((other) => other !== tag); }}>
          x
        </button>
      </span>
    {/each}
    <input id="device-tags" bind:value={draft} onkeydown={keydown} onblur={commit} autocomplete="off" />
  </div>
  <small>Press Enter or comma to add a tag.</small>
</div>

<style>
  .field {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
  }

  label,
  small {
    color: var(--text-muted);
    font-size: var(--text-sm);
  }

  small {
    font-size: var(--text-xs);
  }

  .box {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1);
    padding: var(--space-1) var(--space-2);
    background: var(--surface);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-md);
  }

  .box:focus-within {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 0 var(--space-2);
    background: var(--accent-tint);
    border-radius: var(--radius-pill);
    font-size: var(--text-sm);
  }

  .chip button {
    padding: 0;
    background: none;
    border: 0;
    color: var(--text-muted);
    font: inherit;
  }

  input {
    flex: 1;
    min-inline-size: calc(var(--space-16) * 2);
    min-block-size: var(--target-min-desktop);
    background: none;
    border: 0;
    color: var(--text);
    font: inherit;
  }

  input:focus {
    outline: none;
  }
</style>
