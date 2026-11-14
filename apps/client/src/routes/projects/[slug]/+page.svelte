<script lang="ts">
  import { goto } from '$app/navigation';
  import { resolve } from '$app/paths';
  import { page } from '$app/state';
  import { projectsApi } from '$lib/projects/api';
  import ProjectScreen from '$lib/projects/detail/ProjectScreen.svelte';
  import type { ProjectTab } from '$lib/projects/detail/tabs';
  import { ProjectsStore } from '$lib/projects/store.svelte';
  import { useHub } from '$lib/hub/context';

  const store = new ProjectsStore(projectsApi(useHub()));
  const slug = $derived(page.params['slug'] ?? '');
  $effect(() => {
    void store.open(slug);
  });
</script>

<ProjectScreen
  {store}
  {slug}
  tab={page.url.searchParams.get('tab')}
  onTab={(tab: ProjectTab) => {
    const here = resolve('/projects/[slug]', { slug });
    // eslint-disable-next-line svelte/no-navigation-without-resolve -- the path comes from resolve(); only the tab word is added
    void goto(tab === 'overview' ? here : `${here}?tab=${tab}`, { keepFocus: true, noScroll: true });
  }}
/>
