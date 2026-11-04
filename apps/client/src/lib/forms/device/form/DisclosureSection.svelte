<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from '$lib/icons/Icon.svelte';

  /*
   * A plain disclosure with no motion. It has the props of the shared Disclosure (title, open, children)
   * so that `forms/disclosure/Disclosure.svelte` can replace it when the two tasks merge.
   */
  interface Props {
    title: string;
    open: boolean;
    children: Snippet;
  }

  let { title, open = $bindable(), children }: Props = $props();
  const bodyId = $props.id();
</script>

<section>
  <h3>
    <button type="button" aria-expanded={open} aria-controls={bodyId} onclick={() => { open = !open; }}>
      <Icon name={open ? 'nf-md-chevron_down' : 'nf-md-chevron_right'} size="sm" />
      {title}
    </button>
  </h3>
  {#if open}
    <div id={bodyId} class="body">
      {@render children()}
    </div>
  {/if}
</section>

<style>
  section {
    border-top: var(--border-thin) solid var(--border);
    padding-block: var(--space-2);
  }

  h3 {
    margin: 0;
    font-size: var(--text-md);
  }

  h3 button {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    inline-size: 100%;
    min-block-size: var(--target-min-desktop);
    padding: 0;
    background: none;
    border: 0;
    color: var(--text);
    font: inherit;
    font-weight: var(--weight-bold);
    text-align: start;
  }

  h3 button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .body {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding-block: var(--space-2);
  }
</style>
