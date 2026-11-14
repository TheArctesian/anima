<script lang="ts">
  import type { Snippet } from 'svelte';
  import type { IconName } from '$lib/icons/icons';
  import SectionLabel from '$lib/page/SectionLabel.svelte';

  interface Props {
    /** The heading's id; the section is named by it, and the page's index links to it. */
    id: string;
    title: string;
    icon: IconName;
    /** One line under the heading saying what the section is for; the device's Settings tab has none (D17). */
    description?: string | undefined;
    /** The section's rows (`SettingRow`), with a hairline between each. */
    children: Snippet;
  }

  const { id, title, icon, description, children }: Props = $props();
</script>

<!-- One Settings subject (SET-001) in the website's language (UR-7, D6, D9): a small head of icon, rule and words,
     a line saying what it is for, then hairline rows; no card. -->
<section aria-labelledby={id} aria-describedby={description === undefined ? undefined : `${id}-about`} data-section={id}>
  <SectionLabel text={title} {icon} level={2} {id} />
  {#if description !== undefined}<p id="{id}-about">{description}</p>{/if}
  <div class="rows">{@render children()}</div>
</section>

<style>
  section {
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    scroll-margin-block-start: var(--space-6);
  }

  /* Each line is reserved from tokens, so nothing below moves when the font arrives (MOT-007). */
  p {
    margin: 0 0 var(--space-2);
    min-block-size: calc(var(--text-sm) * var(--leading-tight));
    color: var(--text-tertiary);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  /* Every row draws a hairline above itself; the list closes with one under the last. */
  .rows {
    display: flex;
    flex-direction: column;
    border-block-end: var(--border-thin) solid var(--border-subtle);
  }
</style>
