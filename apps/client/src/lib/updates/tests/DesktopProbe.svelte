<script lang="ts">
  import { provideDesktopUpdates } from '../desktop.svelte';
  import { useUpdates } from '../updates.svelte';

  provideDesktopUpdates();
  const updates = useUpdates();
</script>

<!-- Test support: a component that does what the root layout does, so a test can see whether checking starts and how it ends. -->
<p data-update-state>{updates.snapshot.state.kind}</p>
