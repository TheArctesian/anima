<script lang="ts">
  import { page } from '$app/state';
  import { hostedAccount } from '$lib/hosted/choose';
  import HostedSignIn from './HostedSignIn.svelte';

  const account = hostedAccount(import.meta.env['VITE_ANIMA_HOSTED'], page.url.searchParams);
</script>

<HostedSignIn {account} />
