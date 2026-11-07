<!-- Test support: a `SessionView` below a page that provides `hub`, as the app's layout does. -->
<script lang="ts">
  import type { ComponentProps } from 'svelte';
  import { provideHub } from '$lib/hub/context';
  import type { Hub } from '$lib/hub/hub';
  import SessionView from './SessionView.svelte';

  const { hub, ...rest }: { hub: Hub } & ComponentProps<typeof SessionView> = $props();
  // svelte-ignore state_referenced_locally
  provideHub(hub);
</script>

<SessionView {...rest} />
