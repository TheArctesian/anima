<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { IconName } from '$lib/icons/icons';
  import SectionLabel from './SectionLabel.svelte';
  import { pageTitle } from './title';

  interface Props {
    /** The page's name: the visible `h1`, and the window title "<title> – Anima" (NAV-010). */
    title: string;
    /** A secondary line under the title. */
    detail?: string;
    /** A short label over the title, led by an icon: where the page sits (UR-7). */
    eyebrow?: { readonly text: string; readonly icon: IconName } | undefined;
    /** The page's actions, at the trailing end; they move under the title in a narrow window. */
    actions?: Snippet;
  }

  const { title, detail, eyebrow, actions }: Props = $props();
</script>

<svelte:head><title>{pageTitle(title)}</title></svelte:head>

<header>
  <div class="heading">
    {#if eyebrow !== undefined}<SectionLabel text={eyebrow.text} icon={eyebrow.icon} />{/if}
    <h1>{title}</h1>
    {#if detail !== undefined}<p>{detail}</p>{/if}
  </div>
  {#if actions}<div class="actions">{@render actions()}</div>{/if}
</header>

<style>
  header {
    display: flex;
    flex-wrap: wrap;
    align-items: end;
    justify-content: space-between;
    gap: var(--space-3) var(--space-6);
  }

  .heading {
    display: flex;
    flex: 1 1 0;
    flex-direction: column;
    gap: var(--space-2);
    min-inline-size: min(100%, 12ch);
  }

  h1 {
    margin: 0;
    overflow-wrap: anywhere;
    font-size: var(--text-display);
    font-weight: var(--weight-light);
    line-height: var(--leading-tight);
    letter-spacing: var(--tracking-tight);
  }

  p {
    margin: 0;
    color: var(--text-tertiary);
    line-height: var(--leading-normal);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-2);
  }
</style>
