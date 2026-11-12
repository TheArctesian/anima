<script lang="ts">
  import ContainersDialogs from './ContainersDialogs.svelte';
  import ContainersPage from './ContainersPage.svelte';
  import ContainersResults from './ContainersResults.svelte';
  import ContainersStatus from './ContainersStatus.svelte';
  import ContainersTables from './ContainersTables.svelte';
</script>

<!-- Phase 3 Containers lane: every container primitive in every state, on the sunken page colour the cards sit on. -->
<section class="containers" aria-labelledby="specimen-containers">
  <h2 id="specimen-containers">Containers</h2>
  <ContainersPage />
  <ContainersStatus />
  <ContainersTables />
  <ContainersResults />
  <ContainersDialogs />
</section>

<style>
  .containers {
    display: grid;
    gap: var(--space-6);
    padding: var(--space-4);
    background: var(--surface-sunken);
    border-radius: var(--radius-lg);
  }

  h2 {
    margin: 0;
    font-size: var(--text-xl);
  }
</style>
