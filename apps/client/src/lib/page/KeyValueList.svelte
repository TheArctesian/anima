<script lang="ts">
  import Icon from '$lib/icons/Icon.svelte';
  import type { Pair } from './pair';

  interface Props {
    pairs: readonly Pair[];
    /** Where values sit in their column; the labels are always at the start. */
    align?: 'start' | 'end';
  }

  const { pairs, align = 'start' }: Props = $props();
</script>

<!-- Structured text in place of prose (O6): hairline rows (UR-7, D9), a quiet label column led by an icon, values that wrap
     and never overflow. -->
<dl class:end={align === 'end'}>
  {#each pairs as pair (pair.label)}
    <div class="pair">
      <dt>{#if pair.icon !== undefined}<span class="mark"><Icon name={pair.icon} size="sm" /></span>{/if}{pair.label}</dt>
      <dd class:mono={pair.mono}>
        <span class="value">{pair.value}</span>
        {#if pair.copy}{@render pair.copy()}{/if}
      </dd>
    </div>
  {/each}
</dl>

<style>
  dl {
    container-type: inline-size;
    display: flex;
    flex-direction: column;
    margin: 0;
    border-block-start: var(--border-thin) solid var(--border-subtle);
  }

  .pair {
    display: grid;
    grid-template-columns: minmax(8rem, 1fr) minmax(0, 3fr);
    gap: var(--space-1) var(--space-3);
    padding: var(--space-3) var(--space-1);
    border-block-end: var(--border-thin) solid var(--border-subtle);
  }

  dt {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    color: var(--text-tertiary);
    overflow-wrap: anywhere;
  }

  .mark {
    display: inline-flex;
    color: var(--text-quiet);
  }

  dd {
    display: flex;
    align-items: center;
    gap: var(--space-2);
    min-inline-size: 0;
    margin: 0;
  }

  .end dd {
    justify-content: end;
    text-align: end;
  }

  .value {
    min-inline-size: 0;
    overflow-wrap: anywhere;
  }

  .mono .value {
    font-family: var(--font-mono);
    font-size: var(--text-sm);
    word-break: break-all;
  }

  /* A narrow container stacks the value under its label. */
  @container (max-inline-size: 24rem) {
    .pair {
      grid-template-columns: minmax(0, 1fr);
    }

    .end dd {
      justify-content: start;
      text-align: start;
    }
  }
</style>
