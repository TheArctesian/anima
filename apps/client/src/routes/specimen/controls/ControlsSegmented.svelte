<script lang="ts">
  import Segmented from '$lib/forms/segmented/Segmented.svelte';

  const status = [
    { value: 'all', label: 'All', count: 24 },
    { value: 'up', label: 'Online', count: 21 },
    { value: 'down', label: 'Offline', count: 3 },
  ];
  const view = [
    { value: 'grid', label: 'Grid' },
    { value: 'table', label: 'Table' },
  ];
  let chosenStatus = $state('all');
  let chosenView = $state('grid');
</script>

<section aria-labelledby="specimen-segmented">
  <h3 id="specimen-segmented">Segmented control</h3>
  <div class="row">
    <Segmented label="Status" options={status} bind:value={chosenStatus} />
    <Segmented label="View" options={view} bind:value={chosenView} />
    <Segmented label="Status, small" options={status} bind:value={chosenStatus} size="sm" />
    <span class="note">Chosen: {chosenStatus}, {chosenView}. Tab to a group, then use the arrow keys, Home and End.</span>
  </div>
</section>

<style>
  section {
    display: grid;
    gap: var(--space-3);
  }

  h3 {
    margin: 0;
  }

  .row {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: var(--space-3);
  }

  .note {
    color: var(--text-secondary);
    font-size: var(--text-sm);
  }
</style>
