<script lang="ts">
  import '../card/geometry.css';

  /** How many placeholder cards fill the first screen. */
  const COUNT = 8;
</script>

<!-- The card's size from tokens; static, so reduced motion needs no change (5.7.3, Loading). -->
<ul class="grid" aria-hidden="true">
  {#each Array.from({ length: COUNT }, (_, index) => index) as index (index)}
    <li class="card"></li>
  {/each}
</ul>

<style>
  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(var(--device-card-min-inline-size), 1fr));
    gap: var(--space-4);
    margin: 0;
    padding: 0;
    list-style: none;
  }

  .card {
    box-sizing: border-box;
    block-size: var(--device-card-block-size);
    border: var(--border-thin) solid var(--border);
    border-radius: var(--radius-lg);
    background: var(--surface);
  }
</style>
