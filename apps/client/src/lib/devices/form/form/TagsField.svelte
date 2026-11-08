<script lang="ts">
  import { normalizeTags } from '../values';

  interface Props {
    tags: readonly string[];
  }

  let { tags = $bindable() }: Props = $props();
  let draft = $state('');
  /** Read out politely, since a chip appearing or going is silent otherwise. */
  let announcement = $state('');

  function remove(tag: string): void {
    tags = tags.filter((other) => other !== tag);
    announcement = `Removed tag ${tag}`;
  }

  /** Adds the typed text as a tag (trimmed, no duplicates, T1's rules). */
  function commit(): void {
    const added = draft.trim();
    if (added !== '' && !tags.includes(added)) announcement = `Added tag ${added}`;
    if (added !== '') tags = normalizeTags([...tags, draft]);
    draft = '';
  }

  function keydown(event: KeyboardEvent): void {
    if (event.key === 'Enter' || event.key === ',') {
      event.preventDefault();
      commit();
    } else if (event.key === 'Backspace' && draft === '') {
      const last = tags.at(-1);
      if (last !== undefined) remove(last);
    }
  }
</script>

<div class="field">
  <label for="device-tags">Tags</label>
  <div class="box">
    {#each tags as tag (tag)}
      <span class="chip">
        {tag}
        <button type="button" aria-label="Remove tag {tag}" onclick={() => { remove(tag); }}>
          x
        </button>
      </span>
    {/each}
    <input id="device-tags" bind:value={draft} onkeydown={keydown} onblur={commit} autocomplete="off" />
  </div>
  <small>Press Enter or comma to add a tag.</small>
  <span class="announcement" role="status" aria-live="polite">{announcement}</span>
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

  .announcement {
    position: absolute;
    overflow: hidden;
    inline-size: var(--border-thin);
    block-size: var(--border-thin);
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .chip button {
    min-inline-size: var(--target-min-desktop);
    min-block-size: var(--target-min-desktop);
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
