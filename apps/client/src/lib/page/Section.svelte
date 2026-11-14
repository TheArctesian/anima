<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { IconName } from '$lib/icons/icons';
  import SectionLabel from './SectionLabel.svelte';

  interface Props {
    /** The section's heading text. */
    title: string;
    /** The icon that leads the heading (UI-026, UR-7). */
    icon?: IconName | undefined;
    /** One line under the heading saying what the section is for. */
    lede?: string;
    /** A control at the end of the header row (an "Add" button, a HelpTip). */
    action?: Snippet;
    /** The actions below the body, usually "Save changes" and a line of state text (SET-005). */
    footer?: Snippet;
    children: Snippet;
  }

  const { title, icon, lede, action, footer, children }: Props = $props();
  const id = $props.id();
</script>

<!-- A section of a page in the website's language (UR-7, D6, D9): no card; a small head of icon, rule and words, then
     the content, whose hairlines carry the structure. Its height is its content's (UI-027). -->
<section aria-labelledby="{id}-title" aria-describedby={lede === undefined ? undefined : `${id}-lede`}>
  <header>
    <SectionLabel text={title} {icon} level={2} id="{id}-title" />
    {#if action}<div class="action">{@render action()}</div>{/if}
  </header>
  {#if lede !== undefined}<p id="{id}-lede">{lede}</p>{/if}
  <div class="body">{@render children()}</div>
  {#if footer}<footer>{@render footer()}</footer>{/if}
</section>

<style>
  section {
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    gap: var(--space-3);
    color: var(--text);
  }

  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-3);
    min-block-size: var(--control-sm);
  }

  p {
    margin: 0;
    max-inline-size: var(--content-form);
    color: var(--text-tertiary);
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

  /* The state text follows the button; the whole footer wraps rather than overflow a narrow section. */
  footer {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    justify-content: end;
    gap: var(--space-2) var(--space-3);
  }
</style>
