<script lang="ts">
  import ContainersSection from './containers/ContainersSection.svelte';
  import ControlsSection from './controls/ControlsSection.svelte';
</script>

<svelte:head><title>Specimen – Anima</title></svelte:head>

<main class="specimen">
  <h1>Specimen</h1>
  <ControlsSection />
  <ContainersSection />
</main>

<style>
  .specimen {
    display: grid;
    gap: var(--space-8);
    max-inline-size: var(--content-page);
    padding: var(--space-6);
  }
</style>
