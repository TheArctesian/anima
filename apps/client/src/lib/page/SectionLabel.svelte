<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';

  interface Props {
    text: string;
    /** The icon that leads the label, in the accent colour (UI-026). */
    icon?: IconName | undefined;
    /** The heading level, when the label heads a section; a plain label otherwise. */
    level?: 2 | 3 | undefined;
    id?: string | undefined;
    /** The heading can take focus from code (`tabindex="-1"`), for a page opened at its section. */
    focusable?: boolean;
  }

  const { text, icon, level, id, focusable = false }: Props = $props();
</script>

<!-- The section head of the website's language (UR-7, D6): an icon, a short rule, then the words. -->
<div class="label">
  {#if icon !== undefined}<span class="mark"><Icon name={icon} size="sm" /></span>{/if}
  <span class="rule" aria-hidden="true"></span>
  {#if level === undefined}
    <span {id}>{text}</span>
  {:else}
    <svelte:element this={`h${String(level)}`} {id} tabindex={focusable ? -1 : undefined}>{text}</svelte:element>
  {/if}
</div>

<style>
  .label {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-inline-size: 0;
    color: var(--text-tertiary);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
  }

  .mark {
    display: inline-flex;
    color: var(--accent);
  }

  .rule {
    flex: none;
    inline-size: var(--space-8);
    block-size: var(--border-thin);
    background: var(--border-strong);
  }

  h2,
  h3 {
    margin: 0;
    font: inherit;
    color: inherit;
  }

  h2:focus-visible,
  h3:focus-visible {
    outline: var(--border-focus) solid var(--focus-ring);
    outline-offset: var(--border-focus);
    border-radius: var(--radius-sm);
  }
</style>
