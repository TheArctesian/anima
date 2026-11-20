<script lang="ts" module>
  import type { UsageSums } from '../api';

  /** One line of a table of shares: a program and model, or a device. */
  export interface ShareRow {
    readonly key: string;
    readonly name: string;
    /** A second, quieter part of the name: the model. */
    readonly detail?: string;
    /** The state of a device's daemon, in words; `reads` is the working state. */
    readonly state?: { readonly words: string; readonly reads: boolean };
    /** The counts; null for a device that sends none. */
    readonly sums: UsageSums | null;
  }
</script>

<script lang="ts">
  import type { Count } from '../query';
  import { tokens, whole } from '../report/units';
  import { costText } from '../report/words';
  import { countOf, share, totalTokens } from './shape';

  interface Props {
    /** The label over the table, and its accessible name. */
    label: string;
    /** A quiet line beside the label. */
    note: string;
    /** The header of the first column. */
    head: string;
    rows: readonly ShareRow[];
    /** What the share bars count. */
    count: Count;
    /** The key of the picked row; null for none. Only a table with `onPick` has one. */
    picked?: string | null;
    onPick?: (key: string | null) => void;
  }

  const { label, note, head, rows, count, picked = null, onPick }: Props = $props();
  const sum = $derived(rows.reduce((all, row) => all + (row.sums === null ? 0 : countOf(row.sums, count)), 0));
</script>

<!-- The counts of each line, with the share of what the page counts as a bar and as the last number. -->
<div class="block">
  <div class="over"><span class="label">{label}</span><span class="note">{note}</span></div>
  <table>
    <caption class="sr">{label}</caption>
    <thead>
      <tr><th scope="col">{head}</th><th scope="col" class="n">Sessions</th><th scope="col" class="n">Prompts</th><th scope="col" class="n">Tokens</th><th scope="col" class="n">Cost</th></tr>
    </thead>
    <tbody>
      {#each rows as row (row.key)}
        {@const sums = row.sums}
        <tr aria-selected={onPick === undefined ? undefined : picked === row.key}>
          <th scope="row" colspan={sums === null ? 5 : undefined}>
            {#if onPick === undefined}
              <span class="name">{row.name}</span>
            {:else}
              <button type="button" aria-pressed={picked === row.key} onclick={() => { onPick(picked === row.key ? null : row.key); }}>
                <span class="name">{row.name}</span>
              </button>
            {/if}
            {#if row.detail !== undefined}<span class="detail">{row.detail}</span>{/if}
            {#if row.state !== undefined}<span class="state" class:reads={row.state.reads}>{row.state.words}</span>{/if}
          </th>
          {#if sums !== null}
            <td class="n">{whole(sums.sessions)}</td>
            <td class="n">{whole(sums.prompts)}</td>
            <td class="n">{tokens(totalTokens(sums))}</td>
            <td class="n">
              <span class="cost">
                <span class="track" aria-hidden="true"><i style:inline-size="{share(countOf(sums, count), sum) * 100}%"></i></span>
                <span>{costText(sums)}</span>
              </span>
            </td>
          {/if}
        </tr>
      {/each}
    </tbody>
  </table>
</div>

<style>
  .block {
    display: flex;
    flex-direction: column;
    gap: var(--space-2);
    min-inline-size: 0;
    /* In a narrow window the table scrolls inside its block, and the page does not. */
    overflow-x: auto;
  }

  .over {
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    justify-content: space-between;
    gap: var(--space-1) var(--space-3);
  }

  .label,
  thead th {
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
    letter-spacing: var(--tracking-small);
  }

  .note {
    color: var(--text-secondary);
    font-size: var(--text-xs);
  }

  table {
    inline-size: 100%;
    border-collapse: collapse;
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
    font-size: var(--text-sm);
  }

  th,
  td {
    padding: var(--space-3) var(--space-4);
    text-align: start;
  }

  tbody tr {
    border-block-start: var(--border-thin) solid var(--border-subtle);
  }

  tbody tr[aria-selected='true'] {
    background: var(--surface-selected);
  }

  tbody th {
    font-weight: var(--weight-regular);
  }

  /* A number column is as wide as its numbers; the name takes the rest, so a state stays on the line of its device. */
  .n {
    inline-size: 1%;
  }

  .n {
    font-family: var(--font-mono);
    font-variant-numeric: tabular-nums;
    text-align: end;
    white-space: nowrap;
  }

  td.n:not(:last-child) {
    color: var(--text-secondary);
  }

  .name {
    font-weight: var(--weight-medium);
  }

  .detail,
  .state {
    margin-inline-start: var(--space-2);
    color: var(--text-secondary);
  }

  .state.reads {
    color: var(--status-up-text);
  }

  button {
    padding: 0;
    border: 0;
    background: transparent;
    color: inherit;
    font: inherit;
    cursor: pointer;
  }

  button:focus-visible {
    outline: var(--border-focus) solid var(--accent);
    outline-offset: var(--border-focus);
  }

  .cost {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: var(--space-2);
  }

  .track {
    inline-size: var(--space-16);
    block-size: var(--space-1);
    border-radius: var(--radius-sm);
    background: var(--surface-selected);
  }

  .track i {
    display: block;
    block-size: 100%;
    border-radius: var(--radius-sm);
    background: var(--chart-series-1);
  }

  .sr {
    position: absolute;
    overflow: hidden;
    inline-size: 1px;
    block-size: 1px;
    clip-path: inset(50%);
    white-space: nowrap;
  }
</style>
