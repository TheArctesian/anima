<script lang="ts">
  import ControlsButtons from './ControlsButtons.svelte';
  import ControlsFields from './ControlsFields.svelte';
  import ControlsHelptips from './ControlsHelptips.svelte';
  import ControlsInputs from './ControlsInputs.svelte';
  import ControlsSegmented from './ControlsSegmented.svelte';
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
