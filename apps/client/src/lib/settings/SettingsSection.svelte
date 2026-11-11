<script lang="ts">
  import type { Snippet } from 'svelte';
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';

  interface Props {
    /** The heading's id; the section is named by it. */
    id: string;
    title: string;
    icon: IconName;
    /** One line under the heading saying what the section is for. */
    description: string;
    /** The section's rows (`SettingRow`), drawn in one card with a hairline between each. */
    children: Snippet;
  }

  const { id, title, icon, description, children }: Props = $props();
</script>

<!-- One Settings subject (SET-001): a heading with its icon and a line of description, then a card of rows. -->
<section aria-labelledby={id} aria-describedby="{id}-about">
  <header>
    <span class="mark"><Icon name={icon} size="lg" /></span>
    <h2 {id}>{title}</h2>
    <p id="{id}-about">{description}</p>
  </header>
  <div class="card">
    <div class="rows">{@render children()}</div>
  </div>
</section>

<style>
  section {
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
  }

  header {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    column-gap: var(--space-2);
    align-items: center;
  }

  .mark {
    display: inline-flex;
    grid-row: 1;
    color: var(--accent);
  }

  /* Each line is reserved from tokens, so nothing below moves when the font arrives (MOT-007). */
  h2 {
    grid-row: 1;
    margin: 0;
    min-block-size: calc(var(--text-lg) * var(--leading-tight));
    font-size: var(--text-lg);
    font-weight: var(--weight-bold);
    line-height: var(--leading-tight);
  }

  p {
    grid-column: 2;
    margin: 0;
    min-block-size: calc(var(--text-sm) * var(--leading-tight));
    color: var(--text-muted);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  /* Every row draws a hairline above itself; the card clips the first one, so no row needs to know its place. */
  .card {
    container-type: inline-size;
    overflow: clip;
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  .rows {
    display: flex;
    flex-direction: column;
    margin-block-start: calc(-1 * var(--border-thin));
  }
</style>
