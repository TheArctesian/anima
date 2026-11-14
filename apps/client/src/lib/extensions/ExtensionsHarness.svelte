<script lang="ts">
  // Test support: the bar and the Settings section on one store, as the root layout and the Settings page set them up.
  import Nav from '$lib/nav/Nav.svelte';
  import ExtensionsSection from './ExtensionsSection.svelte';
  import { provideExtensions, type ExtensionsStore } from './store.svelte';

  const { store }: { store: ExtensionsStore } = $props();
  // The store is fixed for the life of the harness.
  // svelte-ignore state_referenced_locally
  provideExtensions(store);
</script>

<Nav pathname="/settings" now={new Date('2026-11-13T12:00:00Z')} projectsOn={store.isOn('projects')} />
<ExtensionsSection />
