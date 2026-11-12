<script lang="ts">
  import ControlsButtons from './controls-buttons.svelte';
  import ControlsFields from './controls-fields.svelte';
  import ControlsHelptips from './controls-helptips.svelte';
  import ControlsInputs from './controls-inputs.svelte';
  import ControlsSegmented from './controls-segmented.svelte';
</script>

<!-- Phase 3 Controls lane: every control primitive in every state. -->
<section aria-labelledby="specimen-controls">
  <h2 id="specimen-controls">Controls</h2>
  <div class="card">
    <ControlsButtons />
    <ControlsSegmented />
    <ControlsInputs />
    <ControlsFields />
    <ControlsHelptips />
  </div>
</section>

<style>
  .card {
    display: grid;
    gap: var(--space-6);
    padding: var(--space-6);
    background: var(--surface-raised);
    border-radius: var(--radius-lg);
  }
</style>
