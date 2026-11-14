<script lang="ts">
  import { page } from '$app/state';
  import { projectsApi } from '$lib/projects/api';
  import ProjectScreen from '$lib/projects/ProjectScreen.svelte';
  import { ProjectsStore } from '$lib/projects/store.svelte';
  import { useHub } from '$lib/hub/context';

  const store = new ProjectsStore(projectsApi(useHub()));
  const slug = $derived(page.params['slug'] ?? '');
  $effect(() => {
    void store.open(slug);
  });
</script>

<ProjectScreen {store} {slug} />
