<script lang="ts">
  import type { AccessWarning } from '$lib/devices/api';
  import Icon from '$lib/icons/Icon.svelte';
  import { warningLines } from './actions';

  interface Props {
    warnings: readonly AccessWarning[];
  }

  const { warnings }: Props = $props();
</script>

<!-- Below bp-compact only the icon shows; the word stays for assistive technology (UI-003). The word and the icon say it, not the colour alone; the tooltip lists each warning in words (RA-020). -->
{#if warnings.length > 0}
  <span class="badge" title={warningLines(warnings).join('\n')}>
    <Icon name="carbon:warning-alt" size="sm" /><span class="label">Warning</span>
  </span>
{/if}

<style>
  .badge {
    flex: none;
    box-sizing: border-box;
    display: inline-flex;
    align-items: center;
    gap: var(--space-1);
    padding: 0 var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--status-warning-tint);
    color: var(--status-warning);
    font-size: var(--text-sm);
    line-height: var(--leading-tight);
    white-space: nowrap;
  }

  /* bp-compact: below 768 px (the token's maxPx is exclusive). */
  @media (max-width: 767.98px) {
    .label {
      position: absolute;
      inline-size: 1px;
      block-size: 1px;
      overflow: hidden;
      clip-path: inset(50%);
    }

    .badge {
      position: relative;
    }
  }
</style>
