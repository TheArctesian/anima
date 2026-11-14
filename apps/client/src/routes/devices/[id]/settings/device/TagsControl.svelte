<script lang="ts">
  import { normalizeTags } from '$lib/devices/form/values';
  import Icon from '$lib/icons/Icon.svelte';

  interface Props {
    id: string;
    tags: string[];
    /** Runs after a tag is added or removed: a tag saves at once (D17). */
    onchange: () => void;
    'aria-describedby'?: string | undefined;
  }

  let { id, tags = $bindable(), onchange, 'aria-describedby': describedBy }: Props = $props();
  let typed = $state('');
  /** Read out politely, since a chip appearing or going is silent otherwise. */
  let announcement = $state('');

  function remove(tag: string): void {
    tags = tags.filter((other) => other !== tag);
    announcement = `Removed tag ${tag}`;
    onchange();
  }

  /** Adds the typed text as a tag (trimmed, no duplicates); nothing is sent for empty text or a tag already there. */
  function add(): void {
    const added = typed.trim();
    typed = '';
    if (added === '' || tags.includes(added)) return;
    tags = normalizeTags([...tags, added]);
    announcement = `Added tag ${added}`;
    onchange();
  }

  function keydown(event: KeyboardEvent): void {
    if (event.key === 'Enter' || event.key === ',') {
      event.preventDefault();
      add();
    } else if (event.key === 'Backspace' && typed === '') {
      const last = tags.at(-1);
      if (last !== undefined) remove(last);
    }
  }
</script>

<span class="tags">
  {#each tags as tag (tag)}
    <span class="chip">
      {tag}
      <button type="button" tabindex="0" aria-label="Remove tag {tag}" title="Remove tag {tag}" onclick={() => { remove(tag); }}>
        <Icon name="carbon:close" size="sm" />
      </button>
    </span>
  {/each}
  <input {id} bind:value={typed} onkeydown={keydown} onblur={add} placeholder="Add a tag" autocomplete="off" aria-describedby={describedBy} />
  <span class="announcement" role="status" aria-live="polite">{announcement}</span>
</span>

<style>
  .tags {
    box-sizing: border-box;
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-1);
    min-block-size: var(--control-md);
    padding: var(--border-focus) var(--space-1);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface-base);
  }

  .tags:focus-within {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-thin);
  }

  .chip {
    display: inline-flex;
    align-items: center;
    gap: var(--border-focus);
    block-size: calc(var(--space-5) + var(--border-focus));
    padding: 0 var(--border-focus) 0 var(--space-2);
    border-radius: var(--radius-sm);
    background: var(--surface-raised);
    color: var(--text);
    font-size: var(--text-sm);
  }

  .chip button {
    display: inline-grid;
    place-items: center;
    inline-size: calc(var(--space-4) + var(--border-focus));
    block-size: calc(var(--space-4) + var(--border-focus));
    padding: 0;
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text-tertiary);
    cursor: pointer;
  }

  .chip button:hover {
    color: var(--text);
  }

  .chip button:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
  }

  /* The row's control style is for single fields; this one sits inside the box. */
  .tags input {
    flex: 1;
    min-inline-size: calc(var(--space-16) + var(--space-4));
    block-size: auto;
    padding: 0 var(--space-1);
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    font-size: var(--text-sm);
    text-align: start;
  }

  .tags input:focus {
    outline: none;
  }

  .tags input::placeholder {
    color: var(--text-tertiary);
    font-style: italic;
    opacity: 1;
  }

  .announcement {
    position: absolute;
    overflow: hidden;
    inline-size: var(--border-thin);
    block-size: var(--border-thin);
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
