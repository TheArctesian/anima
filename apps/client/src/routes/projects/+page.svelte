<script lang="ts">
  import { onMount } from 'svelte';
  import { projectsApi } from '$lib/projects/api';
  import ProjectsScreen from '$lib/projects/ProjectsScreen.svelte';
  import { ProjectsStore } from '$lib/projects/store.svelte';
  import { useHub } from '$lib/hub/context';
  import { isOnThisComputer } from '$lib/hub/hub';

  const hub = useHub();
  const store = new ProjectsStore(projectsApi(hub));
  onMount(() => {
    void store.load();
  });
</script>

<ProjectsScreen {store} keyFileOnly={isOnThisComputer(hub.address)} />
