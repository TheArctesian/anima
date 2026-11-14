<script lang="ts">
  import type { Snippet } from 'svelte';
  import { pageTitle } from './title';

  interface Props {
    /** The page's name: the visible `h1`, and the window title "<title> – Anima" (NAV-010). */
    title: string;
    /** A secondary line under the title. */
    detail?: string;
    /** The page's actions, at the trailing end; they move under the title in a narrow window. */
    actions?: Snippet;
  }

  const { title, detail, actions }: Props = $props();
</script>

<svelte:head><title>{pageTitle(title)}</title></svelte:head>

<header>
  <div class="heading">
    <h1>{title}</h1>
    {#if detail !== undefined}<p>{detail}</p>{/if}
  </div>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</header>

<style>
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3) var(--space-4);
  }

  .heading {
    display: flex;
    flex: 1 1 0;
    flex-direction: column;
    gap: var(--space-1);
    min-inline-size: min(100%, 12ch);
  }

  h1 {
    margin: 0;
    overflow-wrap: anywhere;
    font-size: var(--text-2xl);
    font-weight: var(--weight-medium);
    line-height: var(--leading-tight);
    letter-spacing: var(--tracking-display);
  }

  p {
    margin: 0;
    color: var(--text-secondary);
    line-height: var(--leading-normal);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }
</style>
