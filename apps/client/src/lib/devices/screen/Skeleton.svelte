<script lang="ts">
  import '../card/geometry.css';

  interface Props {
    /** The view the data will take: cards, or rows of the table's height. */
    view: 'grid' | 'table';
    /** The data will open with a group title (D18), so the first placeholder is one. */
    grouped?: boolean;
  }

  const { view, grouped = false }: Props = $props();

  /** How many placeholders fill the first screen. */
  const COUNT = 8;
  const places = Array.from({ length: COUNT }, (_, index) => index);
</script>

<!-- The sizes come from tokens; static, so reduced motion needs no change (5.7.3, Loading). -->
{#if view === 'table'}
  <!-- A table with the data table's header row, group title and row size, so each row lands where its placeholder was (MOT-007). -->
  <table class="places table" aria-hidden="true">
    <caption class="caption"></caption>
    <thead>
      <tr><th></th></tr>
    </thead>
    <tbody>
      {#if grouped}<tr class="head"><th></th></tr>{/if}
      {#each places as place (place)}
        <tr class="place"><td></td></tr>
      {/each}
    </tbody>
  </table>
{:else}
  <ul class="places grid" aria-hidden="true">
    {#if grouped}<li class="head"></li>{/if}
    {#each places as place (place)}
      <li class="place"></li>
    {/each}
  </ul>
{/if}

<style>
  ul.places {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  /* The data grid's columns and gap (DeviceGrid). */
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(min(100%, var(--device-card-min-inline-size)), 1fr));
    gap: var(--space-3);
  }

  /* The first group title of the grid: no space above it (DeviceGrid). */
  .grid .head {
    grid-column: 1 / -1;
    min-block-size: var(--control-lg);
    padding-block-end: var(--space-2);
    border-block-end: var(--border-thin) solid var(--border-subtle);
  }

  .grid .place {
    box-sizing: border-box;
    block-size: var(--device-card-block-size);
    border: var(--border-thin) solid var(--border-strong);
    border-radius: var(--radius-lg);
    background: var(--surface-raised);
  }

  /* The data table has a caption out of the flow; rows sit half a border lower in a table that has one. */
  .caption {
    position: absolute;
  }

  table.places {
    inline-size: 100%;
    border-collapse: collapse;
  }

  .table th,
  .table td {
    box-sizing: border-box;
    padding: 0;
    border-bottom: var(--border-thin) solid var(--border-strong);
  }

  /* The column heads and the first group title of the list are one control high (rows.css). */
  .table th {
    block-size: var(--control-md);
  }

  /* The data's column heads have no rule under them; the group title has a quiet one. */
  .table thead th {
    border-bottom: 0;
  }

  .table .head th {
    border-bottom-color: var(--border-subtle);
  }

  .table td {
    block-size: var(--device-row-block-size);
    background: var(--surface-raised);
  }
</style>
