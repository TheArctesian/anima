<script lang="ts">
  import ContainersDialogs from './containers-dialogs.svelte';
  import ContainersPage from './containers-page.svelte';
  import ContainersResults from './containers-results.svelte';
  import ContainersStatus from './containers-status.svelte';
  import ContainersTables from './containers-tables.svelte';
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
