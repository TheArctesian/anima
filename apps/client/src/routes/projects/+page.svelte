<script lang="ts">
  import { projectsApi } from '$lib/projects/api';
  import ProjectsScreen from '$lib/projects/ProjectsScreen.svelte';
  import { ProjectsStore } from '$lib/projects/store.svelte';
  import { useHub } from '$lib/hub/context';

  const store = new ProjectsStore(projectsApi(useHub()));
  $effect(() => {
    void store.load();
  });
</script>

<ProjectsScreen {store} />
