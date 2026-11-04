<script lang="ts">
  import '../card/geometry.css';

  interface Props {
    /** The view the data will take: cards, or rows of the table's height. */
    view: 'grid' | 'table';
  }

  const { view }: Props = $props();

  /** How many placeholders fill the first screen. */
  const COUNT = 8;
  const places = Array.from({ length: COUNT }, (_, index) => index);
</script>

<!-- The sizes come from tokens; static, so reduced motion needs no change (5.7.3, Loading). -->
<ul class="places {view}" aria-hidden="true">
  {#each places as place (place)}
    <li class="place"></li>
  {/each}
</ul>

<style>
  .places {
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--device-card-min-inline-size), 1fr));
    gap: var(--space-4);
  }

  .grid .place {
    box-sizing: border-box;
    block-size: var(--device-card-block-size);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }

  .table .place {
    box-sizing: border-box;
    block-size: var(--target-min-touch);
    border-bottom: var(--border-thin) solid var(--border);
    background: var(--surface);
  }
</style>
