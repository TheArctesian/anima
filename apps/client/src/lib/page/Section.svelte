<script lang="ts">
  import type { Snippet } from 'svelte';

  interface Props {
    /** The section's heading text. */
    title: string;
    /** One line under the heading saying what the section is for. */
    lede?: string;
    /** A control at the end of the header row (an "Add" button, a HelpTip). */
    action?: Snippet;
    /** The actions below the body, usually "Save changes" and a line of state text (SET-005). */
    footer?: Snippet;
    /** The body has no padding: for rows (`SettingRow`) or a table that run to the card's edges. */
    flush?: boolean;
    children: Snippet;
  }

  const { title, lede, action, footer, flush = false, children }: Props = $props();
  const id = $props.id();
</script>

<!-- A card on the sunken page: raised surface, elevation-1, no border (MAT-006 adds one under prefers-contrast). Its height is its content's (UI-027). -->
<section aria-labelledby="{id}-title" aria-describedby={lede === undefined ? undefined : `${id}-lede`}>
  <header>
    <div class="heading">
      <h2 id="{id}-title">{title}</h2>
      {#if lede !== undefined}<p id="{id}-lede">{lede}</p>{/if}
    </div>
    {#if action}<div class="action">{@render action()}</div>{/if}
  </header>
  <div class="body" class:flush>{@render children()}</div>
  {#if footer}<footer>{@render footer()}</footer>{/if}
</section>

<style>
  section {
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    padding-block: var(--space-4);
    background: var(--surface-raised);
    border-radius: var(--radius-lg);
    box-shadow: var(--elevation-1);
    color: var(--text);
  }

  header,
  footer,
  .body:not(.flush) {
    padding-inline: var(--space-4);
  }

  header {
    display: flex;
    align-items: start;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .heading {
    display: flex;
    flex-direction: column;
    gap: var(--space-1);
    min-inline-size: 0;
  }

  h2 {
    margin: 0;
    font-size: var(--text-lg);
    font-weight: var(--weight-medium);
    line-height: var(--leading-tight);
  }

  p {
    margin: 0;
    color: var(--text-secondary);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  .action {
    flex: none;
    display: flex;
    align-items: center;
    gap: var(--space-2);
  }

  .body {
    min-inline-size: 0;
  }

  /* The state text follows the button; the whole footer wraps rather than overflow a narrow card. */
  footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: end;
    gap: var(--space-2) var(--space-3);
  }

  @media (prefers-contrast: more) {
    section {
      outline: var(--border-thin) solid var(--border-strong);
      outline-offset: calc(var(--border-thin) * -1);
    }
  }
</style>
