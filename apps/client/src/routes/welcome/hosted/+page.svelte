<script lang="ts">
  import { page } from '$app/state';
  import { hostedParts } from '$lib/hosted/choose';
  import HostedSignIn from './HostedSignIn.svelte';

  const parts = hostedParts(import.meta.env['VITE_ANIMA_HOSTED'], page.url.searchParams);
</script>

<HostedSignIn {parts} />
