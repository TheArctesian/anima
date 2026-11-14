<script lang="ts" generics="Row">
  import type { Snippet } from 'svelte';
  import {
    SEVERITY_WORDS,
    columnWidth,
    tableMinWidth,
    withGroupHeaders,
    type Column,
  } from './table';

  interface Props {
    columns: readonly Column<Row>[];
    rows: readonly Row[];
    /** Keeps a row's element when the rows change. */
    rowKey: (row: Row) => string;
    /** Names the table for assistive technology; it is not drawn. */
    caption: string;
    density?: 'comfortable' | 'dense';
    /** A group label per row (see `dayLabel`); a header row is drawn wherever it changes. Rows must already be in group order. */
    groupOf?: (row: Row) => string;
    /** Draws a `custom` column's cell. */
    cell?: Snippet<[Row, Column<Row>]>;
  }

  const { columns, rows, rowKey, caption, density = 'comfortable', groupOf, cell }: Props = $props();
  const entries = $derived(withGroupHeaders(rows, groupOf));
</script>

<!-- A real table: the header cells are `scope="col"`, group headers are `scope="rowgroup"`. Columns are fixed-width or flexible, so it fits without a horizontal scroll bar (DEV-007). -->
<table class={density} style:min-inline-size={tableMinWidth(columns)}>
  <caption>{caption}</caption>
  <colgroup>
    {#each columns as column (column.key)}<col style:width={columnWidth(column)} />{/each}
  </colgroup>
  <thead>
    <tr>
      {#each columns as column (column.key)}
        <th scope="col" class:number={column.kind === 'number'}>{column.header}</th>
      {/each}
    </tr>
  </thead>
  <tbody>
    {#each entries as entry ('group' in entry ? `group:${entry.group}` : rowKey(entry.row))}
      {#if 'group' in entry}
        <tr class="group"><th scope="rowgroup" colspan={columns.length}>{entry.group}</th></tr>
      {:else}
        {@const row = entry.row}
        <tr>
          {#each columns as column (column.key)}
            {@const text = column.value?.(row) ?? ''}
            {#if column.kind === 'custom'}
              <td>{@render cell?.(row, column)}</td>
            {:else if column.kind === 'severity'}
              {@const level = column.severity?.(row) ?? 'info'}
              <td><span class="severity {level}"><span class="dot" aria-hidden="true"></span>{SEVERITY_WORDS[level]}</span></td>
            {:else if column.kind === 'number'}
              <td class="number">{text}</td>
            {:else}
              <td class="text" title={text}>{text}</td>
            {/if}
          {/each}
        </tr>
      {/if}
    {/each}
  </tbody>
</table>

<style>
  table {
    inline-size: 100%;
    table-layout: fixed;
    border-collapse: separate;
    border-spacing: 0;
    color: var(--text);
  }

  caption {
    position: absolute;
    inline-size: 1px;
    block-size: 1px;
    overflow: hidden;
    clip-path: inset(50%);
  }

  th,
  td {
    box-sizing: border-box;
    padding: 0 var(--space-3);
    border-block-end: var(--border-thin) solid var(--border-subtle);
    text-align: start;
    vertical-align: middle;
  }

  /* Comfortable rows are `control-lg` tall; dense rows are `control-sm` (UI-013). */
  .comfortable :is(th, td) {
    block-size: var(--control-lg);
  }

  .dense :is(th, td) {
    block-size: var(--control-sm);
  }

  thead th {
    position: sticky;
    inset-block-start: 0;
    z-index: var(--layer-sticky);
    background: var(--surface-raised);
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  .group th {
    background: var(--surface-sunken);
    color: var(--text-secondary);
    font-size: var(--text-sm);
    font-weight: var(--weight-medium);
  }

  tbody tr:not(.group):hover {
    background: var(--surface-hover);
  }

  /* A long text is cut with an ellipsis; the full text is the cell's title (UI-027). */
  .text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Numbers line up on the right in tabular figures (UI-025). */
  .number {
    font-variant-numeric: tabular-nums;
    text-align: end;
    white-space: nowrap;
  }

  .severity {
    display: inline-flex;
    align-items: center;
    gap: var(--space-2);
  }

  .dot {
    inline-size: var(--space-2);
    block-size: var(--space-2);
    border-radius: var(--radius-pill);
    background: var(--tone);
  }

  .info {
    --tone: var(--status-unknown);
  }

  .warning {
    --tone: var(--status-warning);
  }

  .error {
    --tone: var(--status-down);
  }
</style>
