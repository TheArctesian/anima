<script lang="ts">
  import type { AccessWarning } from '$lib/devices/api';
  import Icon from '$lib/icons/Icon.svelte';
  import HelpTip from '$lib/overlay/helptip/HelpTip.svelte';
  import { WARNING_MEANING, warningLines, warningName } from './actions';

  interface Props {
    warnings: readonly AccessWarning[];
  }

  const { warnings }: Props = $props();
</script>

<!--
  The badge opens its own tip on hover, focus or a click: what Warning means, then each warning in words (RA-020). Its
  spoken name is the word, the meaning and each warning, so a screen reader hears all of it without opening the tip,
  which comes and goes with its content and is not reliably announced. Below bp-compact
  only the icon shows (UI-003); the word and the icon say it, not the colour alone.
-->
{#if warnings.length > 0}
  <HelpTip label={warningName(warnings)}>
    {#snippet face()}
      <span class="badge"><Icon name="carbon:warning-alt" size="sm" /><span class="label">Warning</span></span>
    {/snippet}
    <p>{WARNING_MEANING}</p>
    <ul>
      {#each warningLines(warnings) as line (line)}<li>{line}</li>{/each}
    </ul>
  </HelpTip>
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
    /* The text colour of the status, as in the mockup: small text on the tint needs it (WCAG 1.4.3). */
    color: var(--status-warning-text);
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
