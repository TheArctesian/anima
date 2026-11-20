<script lang="ts">
  import type { UsageSums } from '../api';
  import type { Count } from '../query';
  import { tokens, whole } from '../report/units';
  import { costText, estimateNote } from '../report/words';
  import { shortDay, tokenParts, totalTokens, type DayBar } from './shape';

  interface Props {
    total: UsageSums;
    pricesAsOf: string;
    /** One bar for each UTC day of the page, in order. */
    bars: readonly DayBar[];
    count: Count;
    /** The program and model that the bars are limited to, in words; empty for all. */
    only: string;
  }

  const { total, pricesAsOf, bars, count, only }: Props = $props();
  const CHART: Readonly<Record<Count, string>> = { cost: 'Estimated cost', tokens: 'Tokens', sessions: 'Sessions' };
  const parts = $derived(tokenParts(total));
  const chart = $derived(`${CHART[count]} for each day (UTC)${only === '' ? '' : `, ${only} only`}`);
  const first = $derived(bars[0]?.day);
  const last = $derived(bars.at(-1)?.day);

  function barValue(value: number): string {
    if (count === 'cost') return costText({ estimatedCostUsd: value });
    return count === 'tokens' ? `${tokens(value)} tokens` : `${whole(value)} sessions`;
  }
</script>

<!-- The totals of the choices of the page, one bar for each day, and the tokens by kind (AIS-020, AIS-021). Each figure
     is also text: the bars carry their day and their value for a pointer and for a screen reader. -->
<section class="totals" aria-label="Totals">
  <dl class="figures">
    <div><dt>Sessions</dt><dd>{whole(total.sessions)}</dd></div>
    <div><dt>Prompts</dt><dd>{whole(total.prompts)}</dd></div>
    <div><dt>Tool calls</dt><dd>{whole(total.toolCalls)}</dd></div>
    <div><dt>Tokens</dt><dd>{tokens(totalTokens(total))}</dd><dd class="note">all types</dd></div>
    <div><dt>Cost, estimate</dt><dd>{costText(total)}</dd><dd class="note">{estimateNote(shortDay(pricesAsOf), total.unpricedTokens)}</dd></div>
  </dl>
  <ol class="days" aria-label={chart}>
    {#each bars as bar (bar.day)}
      <li class:none={bar.value === 0} style:--share={bar.height} title="{bar.day}: {barValue(bar.value)}">
        <span class="sr">{bar.day}: {barValue(bar.value)}</span>
      </li>
    {/each}
  </ol>
  <div class="axis">
    <span>{first === undefined ? '' : shortDay(first)}</span><span>{chart}</span><span>{last === undefined ? '' : shortDay(last)}</span>
  </div>
  <div class="kinds">
    <div class="bar" role="img" aria-label="Tokens by type">
      {#each parts as part, index (part.label)}<i style:flex-grow={part.tokens} data-series={index + 1}></i>{/each}
    </div>
    <ul>
      {#each parts as part (part.label)}<li>{part.label} {tokens(part.tokens)}</li>{/each}
    </ul>
  </div>
</section>

<style>
  .totals {
    display: flex;
    flex-direction: column;
    gap: var(--space-4);
    padding: var(--space-4);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  .figures {
    display: grid;
    grid-template-columns: repeat(5, minmax(0, 1fr));
    gap: var(--space-4);
    margin: 0;
  }

  .figures div {
    display: flex;
    flex-direction: column;
    gap: var(--space-0);
    min-inline-size: 0;
  }

  dt {
    color: var(--text-secondary);
    font-size: var(--text-xs);
    font-weight: var(--weight-medium);
    letter-spacing: var(--tracking-label);
    text-transform: uppercase;
  }

  dd {
    margin: 0;
    font-family: var(--font-mono);
    font-size: var(--text-xl);
    font-weight: var(--weight-medium);
    font-variant-numeric: tabular-nums;
  }

  dd.note,
  .axis,
  .kinds ul {
    color: var(--text-secondary);
    font-family: inherit;
    font-size: var(--text-xs);
    font-weight: var(--weight-regular);
  }

  .days {
    display: flex;
    align-items: flex-end;
    gap: var(--border-focus);
    block-size: calc(var(--space-12) * 2);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .days li {
    flex: 1;
    min-block-size: var(--border-focus);
    block-size: calc(var(--share) * 100%);
    border-radius: var(--radius-sm) var(--radius-sm) 0 0;
    background: var(--chart-series-1);
  }

  .days li.none {
    background: var(--surface-selected);
  }

  .sr {
    position: absolute;
    overflow: hidden;
    inline-size: 1px;
    block-size: 1px;
    clip-path: inset(50%);
    white-space: nowrap;
  }

  .axis {
    display: flex;
    justify-content: space-between;
    gap: var(--space-3);
  }

  .kinds {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
  }

  .bar {
    display: flex;
    overflow: hidden;
    block-size: var(--space-2);
    border-radius: var(--radius-md);
    background: var(--surface-selected);
  }

  .bar i[data-series='1'] {
    background: var(--chart-series-1);
  }

  .bar i[data-series='2'] {
    background: var(--chart-series-2);
  }

  .bar i[data-series='3'] {
    background: var(--chart-series-3);
  }

  .bar i[data-series='4'] {
    background: var(--chart-series-4);
  }

  .kinds ul {
    display: flex;
    flex-wrap: wrap;
    gap: var(--space-1) var(--space-4);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  @media (width <= 60rem) {
    .figures {
      grid-template-columns: repeat(2, minmax(0, 1fr));
    }
  }
</style>
