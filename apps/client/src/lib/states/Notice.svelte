<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import type { IconName } from '$lib/icons/icons';
  import { noticeRole, type NoticeTone } from './notice';

  interface Props {
    tone: NoticeTone;
    /** What happened, in a few words: "Saved", "Test failed". */
    title: string;
    /** One line of detail: what it means or what to do. */
    line?: string;
  }

  const { tone, title, line }: Props = $props();

  const ICONS: Record<NoticeTone, IconName> = {
    info: 'carbon:information',
    success: 'carbon:checkmark-outline',
    warning: 'carbon:warning-alt',
    error: 'carbon:close-filled',
  };
</script>

<!-- A result inside a section (SET-005): an icon and the title, so the tone never relies on colour (UI-003). Errors interrupt (`alert`); the rest are polite (`status`). -->
<div class="notice {tone}" role={noticeRole(tone)}>
  <span class="mark"><Icon name={ICONS[tone]} size="sm" /></span>
  <p class="title">{title}</p>
  {#if line !== undefined}<p class="line">{line}</p>{/if}
</div>

<style>
  .notice {
    display: grid;
    grid-template-columns: auto minmax(0, 1fr);
    column-gap: var(--space-2);
    row-gap: var(--space-1);
    padding: var(--space-2) var(--space-3);
    border-radius: var(--radius-md);
    background: var(--wash);
    color: var(--text);
    font-size: var(--text-sm);
    line-height: var(--leading-normal);
  }

  .mark {
    display: inline-flex;
    align-items: center;
    grid-row: 1;
    color: var(--tone);
  }

  p {
    margin: 0;
    min-inline-size: 0;
    overflow-wrap: anywhere;
  }

  .title {
    font-weight: var(--weight-medium);
  }

  .line {
    grid-column: 2;
    color: var(--text-secondary);
  }

  .info {
    --tone: var(--accent);
    --wash: var(--accent-tint);
  }

  .success {
    --tone: var(--status-up);
    --wash: var(--status-up-tint);
  }

  .warning {
    --tone: var(--status-warning);
    --wash: var(--status-warning-tint);
  }

  .error {
    --tone: var(--status-down);
    --wash: var(--status-down-tint);
  }
</style>
